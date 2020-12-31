export PATH := $(DEVKITARM)/bin:$(PATH)

PROJECT_NAME=flac-demo
BUILD_TYPE := release
# or debug

BUILD_ARG =
ifeq ($(BUILD_TYPE),release)
BUILD_ARG = --$(BUILD_TYPE)
endif

CARGO_TARGET_PATH=target
TARGET=armv4t-none-eabi
ELF_OUTPUT=$(CARGO_TARGET_PATH)/$(TARGET)/$(BUILD_TYPE)/$(PROJECT_NAME)
ROM_OUTPUT=$(CARGO_TARGET_PATH)/$(PROJECT_NAME)-$(BUILD_TYPE).gba
EMU := mgba-qt -3 -C interframeBlending=1 --log-level 15

SOURCES=$(shell find src internal -name \*.rs)

all: $(ROM_OUTPUT)

test: $(ROM_OUTPUT)
	$(EMU) $(ROM_OUTPUT)

debug: $(ROM_OUTPUT)
	cp $(ELF_OUTPUT) $(ROM_OUTPUT).elf
	$(EMU) $(ROM_OUTPUT).elf

$(ELF_OUTPUT): $(SOURCES) rt0.s Cargo.toml linker_script.ld
	cargo build $(BUILD_ARG)

$(ROM_OUTPUT): $(ELF_OUTPUT)
	arm-none-eabi-objcopy -O binary $(ELF_OUTPUT) $(ROM_OUTPUT)
	gbafix $(ROM_OUTPUT)

clean:
	cargo clean
