MEMORY
{
  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 128K
}

ENTRY(Reset)

SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    KEEP(*(.vector_table))
  } > FLASH

  .text :
  {
    *(.text.Reset)
    *(.text .text.*)
    *(.rodata .rodata.*)
  } > FLASH

  .bss (NOLOAD) :
  {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  } > RAM

  .noinit (NOLOAD) :
  {
    *(.noinit .noinit.*)
  } > RAM

  __stack_top = ORIGIN(RAM) + LENGTH(RAM);
}
