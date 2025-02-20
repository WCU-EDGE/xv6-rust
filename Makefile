# Output directory
OUTDIR = target/kernel

# Setting this to true will dump the asm, which takes way longer and runs even when simply running qemu.
# A clean build with this enabled took 36 seconds, while disabling it only took 18 seconds.
# So simply running the kernel will take around 16 seconds if you enable this.
DUMP_ASM = false

# Setting this to true will strip binary files and optimize builds.
RELEASE = false

# Cross-compiling (e.g., on Mac OS X)
# TOOLPREFIX = i386-jos-elf

# Using native tools (e.g., on X86 Linux)
#TOOLPREFIX =

$(shell mkdir -p $(OUTDIR))

# Try to infer the correct TOOLPREFIX if not set
ifndef TOOLPREFIX
TOOLPREFIX := $(shell if i386-jos-elf-objdump -i 2>&1 | grep '^elf32-i386$$' >/dev/null 2>&1; \
	then echo 'i386-jos-elf-'; \
	elif objdump -i 2>&1 | grep 'elf32-i386' >/dev/null 2>&1; \
	then echo ''; \
	else echo "***" 1>&2; \
	echo "*** Error: Couldn't find an i386-*-elf version of GCC/binutils." 1>&2; \
	echo "*** Is the directory with i386-jos-elf-gcc in your PATH?" 1>&2; \
	echo "*** If your i386-*-elf toolchain is installed with a command" 1>&2; \
	echo "*** prefix other than 'i386-jos-elf-', set your TOOLPREFIX" 1>&2; \
	echo "*** environment variable to that prefix and run 'make' again." 1>&2; \
	echo "*** To turn off this error, run 'gmake TOOLPREFIX= ...'." 1>&2; \
	echo "***" 1>&2; exit 1; fi)
endif

# If the makefile can't find QEMU, specify its path here
# QEMU = qemu-system-i386

# Try to infer the correct QEMU
ifndef QEMU
QEMU = $(shell if which qemu > /dev/null; \
	then echo qemu; exit; \
	elif which qemu-system-i386 > /dev/null; \
	then echo qemu-system-i386; exit; \
	elif which qemu-system-x86_64 > /dev/null; \
	then echo qemu-system-x86_64; exit; \
	else \
	qemu=/Applications/Q.app/Contents/MacOS/i386-softmmu.app/Contents/MacOS/i386-softmmu; \
	if test -x $$qemu; then echo $$qemu; exit; fi; fi; \
	echo "***" 1>&2; \
	echo "*** Error: Couldn't find a working QEMU executable." 1>&2; \
	echo "*** Is the directory containing the qemu binary in your PATH" 1>&2; \
	echo "*** or have you tried setting the QEMU variable in Makefile?" 1>&2; \
	echo "***" 1>&2; exit 1)
endif

CC = $(TOOLPREFIX)gcc
AS = $(TOOLPREFIX)gas
LD = $(TOOLPREFIX)ld
OBJCOPY = $(TOOLPREFIX)objcopy
OBJDUMP = $(TOOLPREFIX)objdump
CFLAGS = -fno-pic -static -fno-builtin -fno-strict-aliasing -O2 -Wall -MD -ggdb -m32 -Werror -fno-omit-frame-pointer
CFLAGS += $(shell $(CC) -fno-stack-protector -E -x c /dev/null >/dev/null 2>&1 && echo -fno-stack-protector)
CFLAGS += $(CS333_CFLAGS)
ASFLAGS = -m32 -gdwarf-2 -Wa,-divide
# FreeBSD ld wants ``elf_i386_fbsd''
LDFLAGS += -m $(shell $(LD) -V | grep elf_i386 2>/dev/null | head -n 1)

# Set Rust target path
RUST_TARGET_PATH := ${CURDIR}

CARGO := @cargo

# Disable PIE when possible (for Ubuntu 16.10 toolchain)
ifneq ($(shell $(CC) -dumpspecs 2>/dev/null | grep -e '[^f]no-pie'),)
CFLAGS += -fno-pie -no-pie
endif
ifneq ($(shell $(CC) -dumpspecs 2>/dev/null | grep -e '[^f]nopie'),)
CFLAGS += -fno-pie -nopie
endif

ifeq ($(RELEASE),true)
RUST_OS := target/i386/release/libxv6.a
else
RUST_OS := target/i386/debug/libxv6.a
endif

xv6.img: bootblock kernel
	$(info xv6.img:)
	dd if=/dev/zero of=$(OUTDIR)/xv6.img count=10000
	dd if=$(OUTDIR)/bootblock of=$(OUTDIR)/xv6.img conv=notrunc
	dd if=$(OUTDIR)/kernel of=$(OUTDIR)/xv6.img seek=1 conv=notrunc

xv6memfs.img: bootblock kernelmemfs
	$(info xv6memfs.img:)
	dd if=/dev/zero of=$(OUTDIR)/xv6memfs.img count=10000
	dd if=$(OUTDIR)/bootblock of=$(OUTDIR)/xv6memfs.img conv=notrunc
	dd if=$(OUTDIR)/kernelmemfs of=$(OUTDIR)/xv6memfs.img seek=1 conv=notrunc

bootblock: bootasm.S bootmain.c
	$(info bootblock:)
	$(CC) $(CFLAGS) -fno-pic -O -nostdinc -I. -c bootmain.c -o $(OUTDIR)/bootmain.o
	$(CC) $(CFLAGS) -fno-pic -nostdinc -I. -c bootasm.S -o $(OUTDIR)/bootasm.o
	$(LD) $(LDFLAGS) -N -e start -Ttext 0x7C00 -o $(OUTDIR)/bootblock.o $(OUTDIR)/bootasm.o $(OUTDIR)/bootmain.o
ifeq ($(DUMP_ASM),true)
	$(OBJDUMP) -S $(OUTDIR)/bootblock.o > $(OUTDIR)/bootblock.asm
endif
	$(OBJCOPY) -S -O binary -j .text $(OUTDIR)/bootblock.o $(OUTDIR)/bootblock
	./sign.pl $(OUTDIR)/bootblock

entry.o: entry.S
	$(info entry.o:)
	$(CC) $(ASFLAGS) -c -o $(OUTDIR)/entry.o entry.S

entryother: entryother.S
	$(info entryother:)
	$(CC) $(CFLAGS) -fno-pic -nostdinc -I. -c entryother.S -o $(OUTDIR)/entryother.o # changed this last part
	$(LD) $(LDFLAGS) -N -e start -Ttext 0x7000 -o $(OUTDIR)/bootblockother.o $(OUTDIR)/entryother.o
	$(OBJCOPY) -S -O binary -j .text $(OUTDIR)/bootblockother.o $(OUTDIR)/entryother
ifeq ($(DUMP_ASM),true)
	$(OBJDUMP) -S $(OUTDIR)/bootblockother.o > $(OUTDIR)/entryother.asm
endif

kernel: rkernel entry.o entryother kernel.ld
	$(info kernel:)
	$(LD) $(LDFLAGS) -T kernel.ld -o $(OUTDIR)/kernel $(OUTDIR)/entry.o $(RUST_OS) -b binary $(OUTDIR)/entryother
ifeq ($(DUMP_ASM),true)
	$(OBJDUMP) -S $(OUTDIR)/kernel > $(OUTDIR)/kernel.asm
	$(OBJDUMP) -t kernel | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > $(OUTDIR)/kernel.sym
endif

rkernel:
	$(info rkernel:)
ifeq ($(RELEASE),true)
	$(CARGO) build -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem --target i386.json --release
else
	$(CARGO) build -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem --target i386.json
endif

docs:
	$(info docs:)
	$(CARGO) doc -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem --target i386.json

docs-open:
	$(info docs:)
	$(CARGO) doc --open -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem --target i386.json

# kernelmemfs is a copy of kernel that maintains the
# disk image in memory instead of writing to a disk.
# This is not so useful for testing persistent storage or
# exploring disk buffering implementations, but it is
# great for testing the kernel on real hardware without
# needing a scratch disk.
MEMFSOBJS = $(filter-out ide.o,) memide.o
kernelmemfs: $(MEMFSOBJS) rkernel entry.o entryother kernel.ld
	$(info kernelmemfs:)
	$(LD) $(LDFLAGS) -T kernel.ld -o $(OUTDIR)/kernelmemfs $(OUTDIR)/entry.o $(MEMFSOBJS) $(RUST_OS) -b binary $(OUTDIR)/entryother
ifeq ($(DUMP_ASM),true)
	$(OBJDUMP) -S kernelmemfs > $(OUTDIR)/kernelmemfs.asm
endif
	$(OBJDUMP) -t kernelmemfs | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > $(OUTDIR)/kernelmemfs.sym

tags: entryother.S
	$(info tags:)
	etags *.S *.c

vectors.S: vectors.pl
	$(info vectors.S:)
	./vectors.pl > vectors.S

ULIB = ulib.o usys.o umalloc.o

# Prevent deletion of intermediate files, e.g. cat.o, after first build, so
# that disk image changes after first build are persistent until clean.  More
# details:
# http://www.gnu.org/software/make/manual/html_node/Chained-Rules.html
.PRECIOUS: %.o

-include *.d

clean:
	$(info clean:)
	rm -r -f $(OUTDIR)
	$(CARGO) clean

ifndef CPUS
CPUS := 2
endif
QEMUOPTS = -drive file=$(OUTDIR)/xv6.img,index=0,media=disk,format=raw -smp $(CPUS) -m 512 $(QEMUEXTRA)

run:
	$(info run:)
	make qemu-nox

debug:
	$(info debug:)
	make qemu-nox-gdb

qemu: xv6.img
	$(info qemu:)
	$(QEMU) -serial mon:stdio $(QEMUOPTS)

qemu-memfs: xv6memfs.img
	$(info qemu-memfs:)
	$(QEMU) -drive file=$(OUTDIR)/xv6memfs.img,index=0,media=disk,format=raw -smp $(CPUS) -m 256

qemu-test-memfs: xv6memfs.img
	$(info qemu-test-memfs:)
	$(QEMU) -display none -serial pipe:vm_fifo -drive file=$(OUTDIR)/xv6memfs.img,index=0,media=disk,format=raw -smp $(CPUS) -m 256

qemu-nox: xv6.img
	$(info qemu-nox:)
	$(QEMU) -nographic $(QEMUOPTS)

qemu-gdb: xv6.img
	$(info qemu-gdb:)
	@echo "Run rust-gdb target/kernel/kernel" 1>&2
	@echo "In rust-gdb run target remote localhost:1234" 1>&2
	$(QEMU) $(QEMUOPTS) -s -S

qemu-nox-gdb: xv6.img
	$(info qemu-nox-gdb:)
	@echo "Run rust-gdb target/kernel/kernel" 1>&2
	@echo "In rust-gdb run target remote localhost:1234" 1>&2
	$(QEMU) -nographic $(QEMUOPTS) -s -S
