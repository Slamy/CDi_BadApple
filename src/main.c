#include <csd.h>
#include <sysio.h>
#include <ucm.h>
#include <events.h>
#include <stdio.h>
#include <setsys.h>
#include <math.h>
#include <memory.h>
#include <cdfm.h>

#include "video.h"
#include "graphics.h"
#include <signal.h>

extern int errno;
int audioPath;

#define DEBUG(c)                         \
	if ((c) == -1)                       \
	{                                    \
		printf("FAIL: c (%d)\n", errno); \
	}

/* 160 * 2324 = 4371840 byte
 * Quite a size, considering just 512kB is available for Plane A
 */
#define VIDEO_PCL_COUNT 160
#define SECTOR_SIZE 2324

int exit_app = 0;
static PCB videoPcb;
static PCL mvPcl[VIDEO_PCL_COUNT];
static PCL *mvCil[32];
char *mpegDataBuffer;
static int mpegFile = -1;

#define VIDEO_SIG_PCB 0x1C00
#define MV_SIG_PCL 0x1B00

int current_pcl = 0;

void initPcb()
{
	int i;

	for (i = 0; i < 32; i++)
	{
		mvCil[i] = (PCL *)mvPcl;
	}

	videoPcb.PCB_Video = mvCil;
	videoPcb.PCB_Audio = NULL; /* Never transfer audio to memory */
	videoPcb.PCB_Data = NULL;
	videoPcb.PCB_Sig = VIDEO_SIG_PCB;
	videoPcb.PCB_Chan = 0x00000003;
	videoPcb.PCB_AChan = 0x00000001;
	videoPcb.PCB_Rec = 1; /* assume that there is only 1 EOR */
	videoPcb.PCB_Stat = 0;
}

void initMpegPcl(pcl, sig, next, buffer, length)
	PCL *pcl; /* pointer to the  PCL to initialise */
short sig;	  /* signal to be sent on buffer full */
PCL *next;	  /* pointer to next PCL */
char *buffer; /* pointer to data buffer */
int length;	  /* buffer size in number of sectors */
{
	pcl->PCL_Sig = sig;
	pcl->PCL_Nxt = next;
	pcl->PCL_Buf = buffer;
	pcl->PCL_BufSz = length;
	pcl->PCL_Ctrl = 0;
	pcl->PCL_Err = NULL;
	pcl->PCL_Cnt = 0;
}

short current_seqnum = 1;
unsigned short min_full_cnt = 0xffff;
unsigned short max_full_cnt = 0;
short phase_mod = 0;

#define EXPECTED_NUMBER_OF_FRAMES 6530
short CountUsedPCLs()
{
	unsigned short full_cnt = 0;
	short i;
	for (i = 0; i < VIDEO_PCL_COUNT; i++)
	{
		if (mvPcl[i].PCL_Ctrl & 0x01)
		{
			full_cnt++;
		}
	}

	/* Ignore the first few frames for analysis of FIFO levels */
	if (current_seqnum > 50 && current_seqnum < (EXPECTED_NUMBER_OF_FRAMES - 20))
	{
		if (full_cnt < min_full_cnt)
			min_full_cnt = full_cnt;
		if (full_cnt > max_full_cnt)
			max_full_cnt = full_cnt;
	}

	return full_cnt;
}

void initPcls()
{
	char *address = mpegDataBuffer;
	int i;

	for (i = 0; i < VIDEO_PCL_COUNT; i++)
	{
		initMpegPcl(
			&(mvPcl[i]),
			MV_SIG_PCL,
			&(mvPcl[(i + 1) % VIDEO_PCL_COUNT]),
			address,
			1);
		address += SECTOR_SIZE;
	}
}

/* Estimated by measuring the video signal and the audio signal
 * using a sound card. The beat drop during the "apple catch invert"
 * is perfect for this.
 * = number of frames *100
 */
int kAudioVideoOffset = 120;

int kSectorRate = 75;
int kVideoFrameRate = 2997; /* *100 */

int mainSignal(sigCode)
int sigCode;
{
	if (sigCode == SIGINT)
	{
		printf("SIGINT!\n");
		exit_app = 1;
	}
	else if (sigCode == VIDEO_SIG_PCB)
	{
		/* Occurs when playback has finished */
		printf("PCB %x %x\n", videoPcb.PCB_Stat, videoPcb.PCB_Sig);
		exit_app = 1;
	}
	else if (sigCode == MV_SIG_PCL)
	{
		if (mpegFile != -1)
		{
			long error;
			int pos = gs_pos(mpegFile) / 2048;
			/* pos counts 75 Hz ticks */
			pos = pos * kVideoFrameRate / kSectorRate + kAudioVideoOffset;
			/* now we have the number of frames *100 */
			error = -(current_seqnum * 100 - pos);

			phase_mod = error;
		}
	}
	else if (sigCode == SIG_BLANK)
	{
		VBlankOccured();
		dc_ssig(videoPath, SIG_BLANK, 0);
	}
	else
	{
	}
}

void initProgram()
{
	mpegDataBuffer = (char *)srqcmem((VIDEO_PCL_COUNT)*SECTOR_SIZE, VIDEO1);
	if (!mpegDataBuffer)
	{
		printf("No memory!\n");
		exit(0);
	}

	initPcls();
	initPcb();
}

void initAudio()
{
	char *devName = csd_devname(DT_AUDIO, 1); /* Get Audio Device Name */
	audioPath = open(devName, UPDAT_);		  /* Open Audio Device */
	free(devName);							  /* Release memory */
	/* NOTE SC_ATTEN( R->L, R->R, L->R, L->L) */
	sc_atten(audioPath, 0x00800080); /* Normal Stereo */
}

void initSystem()
{
	char *path;

	initAudio();
	initVideo();
	initProgram();

	/* Assume we are not running from serial stub first */
	if (videoMode == 0)
		path = "/cd/280p.RTF"; /* PAL - 384x280 */
	else
		path = "/cd/240p.RTF"; /* NTSC TV - 384x240 */

	mpegFile = open(path, _READ);
	DEBUG(mpegFile >= 0);
	DEBUG(lseek(mpegFile, 0, 0));
	DEBUG(ss_play(mpegFile, &videoPcb));
	printf("Started Play %s\n", path);
}

void closeSystem()
{
	closeVideo();
}

#define NEXTFRAME_ENTRIES 16
nextframe_offset nextframe_offsets[NEXTFRAME_ENTRIES];
short nextframe_index = 0;
short nextframe_valid = 0;
short unused_header = 0;

int processSector(unsigned char *buf_current)
{
	unsigned short *header;
	unsigned short magic;
	unsigned short seqnum;
	unsigned short frame_complete;
	unsigned short offset;
	unsigned short length;
	unsigned char *rle_data;

	if (nextframe_valid)
	{
		printf("Consume next frame first\n");
		exit(1);
	}

	for (;;)
	{
		if ((unsigned long)buf_current & 1)
		{
			printf("Alignment error\n");
			exit(1);
		}

		header = buf_current;
		magic = header[0];	/* Last package in sector 0x4242, Not the last one 0x4243 */
		seqnum = header[1] & 0x7fff; /* Frame Index, starting at 1*/
		frame_complete = header[1] & 0x8000;
		offset = header[2]; /* Offset in lines */
		length = header[3]; /* Number of bytes */
		rle_data = &header[4];

		if ((magic & 0xfffe) == 0x4242)
		{
			if (seqnum > current_seqnum)
			{
				current_seqnum = seqnum;
				/* We aren't allowed to continue here, since we have lost our working buffer */
				return 0;
			}

			if (seqnum == current_seqnum)
			{
				if (magic == 0x4242)
				{
					/* We have fully used this sector */
					if (length != 0)
					{
						nextframe_offsets[nextframe_index].adr = rle_data;
						nextframe_offsets[nextframe_index].line = offset;
						nextframe_index++;
					}
					else
					{
						unused_header++;
					}

					if (frame_complete) nextframe_valid = 1;


					if (nextframe_index >= NEXTFRAME_ENTRIES)
					{
						printf("Buffer overflow\n");
						exit(1);
					}

					return 1;
				}
				else if (magic == 0x4243)
				{
					if (length != 0)
					{
						nextframe_offsets[nextframe_index].adr = rle_data;
						nextframe_offsets[nextframe_index].line = offset;
						nextframe_index++;
					}
					else
					{
						unused_header++;
					}

					if (frame_complete) nextframe_valid = 1;

					if (nextframe_index >= NEXTFRAME_ENTRIES)
					{
						printf("Buffer overflow\n");
						exit(1);
					}
					/* Look for another header */
					buf_current = rle_data + length;
				}
			}
			else
			{
				if (magic == 0x4242)
				{
					/* We have fully used this sector */
					return 1;
				}
				else if (magic == 0x4243)
				{
					/* Look for another header */
					buf_current = rle_data + length;
				}
			}
		}
		else
		{
			printf("No magic\n");
			exit(1);
		}
	}
}

void runProgram()
{
	dc_ssig(videoPath, SIG_BLANK, 0);

	while (!exit_app)
	{

		if ((mvPcl[current_pcl].PCL_Ctrl & 1) && !nextframe_valid)
		{
			unsigned char *buf_current = &mpegDataBuffer[current_pcl * SECTOR_SIZE];

			int sector_used_up = processSector(buf_current);
			/* mvPcl[current_pcl].PCL_Buf */
			/* printf("MV %x %x %x\n", videoPcb.PCB_Stat, videoPcb.PCB_Sig, buf_current[0]); */
			/* printf("C %x\n", *(u_short *)buf_current); */

			if (sector_used_up)
			{
				CountUsedPCLs();

				/* printf("C\n"); */
				/* Free PCL */
				initMpegPcl(
					&(mvPcl[current_pcl]),
					MV_SIG_PCL,
					&(mvPcl[(current_pcl + 1) % VIDEO_PCL_COUNT]),
					buf_current,
					1);

				current_pcl = (current_pcl + 1) % VIDEO_PCL_COUNT;
			}
		}
	}
}

extern int os9forkc();
extern char **environ;
char *argblk[] = {
	"vcd",
	0,
};

int main(argc, argv)
int argc;
char *argv[];
{
	int pid;
	intercept(mainSignal);

	initSystem();
	runProgram();
	closeSystem();

	sleep(8);
	printf("Finished... Stats: %d %d\n", min_full_cnt, max_full_cnt);
	exit(0);
}
