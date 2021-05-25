OUTPUT_ARCH("riscv")
ENTRY(_start)

SECTIONS
{
  . = 0x00000000;

  .text :
  {
    *(.text)
    *(trampsec)
  }

  .rodata :
  {
    *(.rodata .rodata.*)
  }

  .data : {
    *(.data .data.*)
  }
  .bss : {
    *(.bss .bss.*)
    *(.sbss* .sbss.*)
  }
}
