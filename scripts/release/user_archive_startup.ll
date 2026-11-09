target triple = "thumbv7em-none-eabihf"

module asm ".syntax unified\0A.cpu cortex-m7\0A.thumb\0A.section .isr_vector,\22a\22,%progbits\0A.section .text.Reset_Handler,\22ax\22,%progbits\0A.global Reset_Handler\0A.type Reset_Handler,%function\0A.thumb_func\0AReset_Handler:\0A  bl boot\0A  cmp r0, #42\0A  bne 1f\0A  ldr r1, =0x20026\0A  b 2f\0A1:\0A  ldr r1, =0x20023\0A2:\0A  movs r0, #0x18\0A  bkpt 0xab\0A3: b 3b\0A"
