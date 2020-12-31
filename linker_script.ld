ENTRY(__start)

MEMORY {
  ewram : ORIGIN = 0x02000000, LENGTH = 256K
  iwram : ORIGIN = 0x03000000, LENGTH = 32K
  rom   : ORIGIN = 0x08000000, LENGTH = 32M
}

SECTIONS {
  .text : {
    KEEP(rt0.o(.text));
    *(.text .text.*);
    . = ALIGN(4);
  } >rom = 0xff

  .rodata : {
    KEEP(rt0.o(.rodata));
    *(.rodata .rodata.*);
    . = ALIGN(4);
  } >rom = 0xff

  __data_rom_start = .;
  .data : {
    __data_iwram_start = ABSOLUTE(.);
    KEEP(rt0.o(.data));
    *(.data .data.*);
    *(.iwram .iwram.*);
    . = ALIGN(4);
    __data_iwram_end = ABSOLUTE(.);
  } >iwram AT>rom = 0xff

  .bss : {
    __bss_iwram_start = ABSOLUTE(.);
    KEEP(rt0.o(.bss));
    *(.bss .bss.*);
    . = ALIGN(4);
    __bss_iwram_end = ABSOLUTE(.);
  } >iwram

  __ewram_rom_start = .;
  .ewram : {
      __ewram_start = ABSOLUTE(.);
      *(.ewram .ewram.*);
      . = ALIGN(4);
      __ewram_end = ABSOLUTE(.);
  } >ewram AT>rom = 0xff

  /* debugging sections */
  /* Stabs */
  .stab            0 : { *(.stab) }
  .stabstr         0 : { *(.stabstr) }
  .stab.excl       0 : { *(.stab.excl) }
  .stab.exclstr    0 : { *(.stab.exclstr) }
  .stab.index      0 : { *(.stab.index) }
  .stab.indexstr   0 : { *(.stab.indexstr) }
  .comment         0 : { *(.comment) }
  /* DWARF 1 */
  .debug           0 : { *(.debug) }
  .line            0 : { *(.line) }
  /* GNU DWARF 1 extensions */
  .debug_srcinfo   0 : { *(.debug_srcinfo) }
  .debug_sfnames   0 : { *(.debug_sfnames) }
  /* DWARF 1.1 and DWARF 2 */
  .debug_aranges   0 : { *(.debug_aranges) }
  .debug_pubnames  0 : { *(.debug_pubnames) }
  /* DWARF 2 */
  .debug_info      0 : { *(.debug_info) }
  .debug_abbrev    0 : { *(.debug_abbrev) }
  .debug_line      0 : { *(.debug_line) }
  .debug_frame     0 : { *(.debug_frame) }
  .debug_str       0 : { *(.debug_str) }
  .debug_loc       0 : { *(.debug_loc) }
  .debug_macinfo   0 : { *(.debug_macinfo) }
  /* SGI/MIPS DWARF 2 extensions */
  .debug_weaknames 0 : { *(.debug_weaknames) }
  .debug_funcnames 0 : { *(.debug_funcnames) }
  .debug_typenames 0 : { *(.debug_typenames) }
  .debug_varnames  0 : { *(.debug_varnames) }
  .got.plt  0 : { *(.got.plt) }

  /* discard anything not already mentioned */
  /DISCARD/ : { *(*) }
}
