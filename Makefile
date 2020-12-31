CARGO_TARGET_PATH=../target
CRT_FILE=crt0.s
CRT_OUTPUT=$(CARGO_TARGET_PATH)/crt0.o
PROJECT_NAME=gba-demo
TARGET=thumbv4-none-agb
THUMB_TARGET=thumbv4-none-agb.json
ELF_OUTPUT=$(CARGO_TARGET_PATH)/$(TARGET)/$(BUILD_TYPE)/$(PROJECT_NAME)
ROM_OUTPUT=$(CARGO_TARGET_PATH)/$(PROJECT_NAME).gba
BUILD_TYPE := release

SOURCES=$(shell find src -name \*.rs)

all: $(ROM_OUTPUT)

$(ELF_OUTPUT): $(SOURCES)
	cp linker.ld ..
	${DEVKITARM}/bin/arm-none-eabi-as $(CRT_FILE) -o $(CRT_OUTPUT)
	cargo xbuild --target $(THUMB_TARGET) --$(BUILD_TYPE)

$(ROM_OUTPUT): $(ELF_OUTPUT)
	${DEVKITARM}/bin/arm-none-eabi-objcopy -O binary $(ELF_OUTPUT) $(ROM_OUTPUT)
	gbafix $(ROM_OUTPUT)

