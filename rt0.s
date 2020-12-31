@ linker entry point
.global __start

@ The linker script places this at the start of the rom.
.arm
__start: b init
  @ this is replaced with correct header info by `gbafix`
  @ note that for mGBA usage gbafix is not required.
  .space (192-4)
@

@ Here we do startup housekeeping to prep for calling Rust
init:
  @ We boot in Supervisor mode.
  @ There's little use for this mode right now,
  @ set System mode.
  mov r0, #0b11111
  msr CPSR_c, r0

  @ copy .data section (if any) to IWRAM
  ldr r0, =__data_rom_start
  ldr r1, =__data_iwram_start
  ldr r2, =__data_iwram_end
  subs r2, r1         @ r2 = data_iwram length (in bytes)
  addne r2, #3        @ round up
  lsrne r2, #2        @ convert r2 to the length in words
  @ addne r2, #(1<<26)  @ set "words" flag
  @ swine 0xB0000       @ call bios::CpuSet
  swine 0xC0000       @ Call bios::CpuFastSet

  @ copy .ewram section (if any) to EWRAM
  ldr r0, =__ewram_rom_start
  ldr r1, =__ewram_start
  ldr r2, =__ewram_end
  subs r2, r1         @ r2 = ewram length (in bytes)
  addne r2, #3        @ round up
  lsrne r2, #2        @ convert r2 to the length in words
  swine 0xC0000       @ Call bios::CpuFastSet

  @ startup complete, branch-exchange to `main`
  ldr r0, =main
  bx r0

  @ `main` should never return, loop if it does.
  1: b 1b
@
