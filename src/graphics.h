#ifndef __GRAPHICS_H__
#define __GRAPHICS_H__

#include "video.h"

extern int curIcfA, curIcfB;
extern unsigned char *paVideo1, *paVideo2;

void CommitFrame(u_char *fb);
void VBlankOccured();
u_char *BorrowFrame();

typedef struct nextframe_offset_s {
    char *adr;
    int line;
} nextframe_offset;

#define NEXTFRAME_ENTRIES 16
extern nextframe_offset nextframe_offsets[NEXTFRAME_ENTRIES];

extern short nextframe_index;
extern short nextframe_valid;
extern short unused_header;

#endif