
MEMORY {
  program (rwx) : ORIGIN = 0x80000000, LENGTH = 10485760
}

SECTIONS {
  .text.boot : {
    *(.text.boot)
  } > program

  .text : {
    *(.text)
  } > program

  .data : {
    *(.data)
  } > program

  .bss : {
    *(.bss)
  } > program

  . = ALIGN(8);
  . = . + 4096;
  _STACK_PTR = .;
  . = ALIGN(8);
  _HEAP_PTR = .;
}
