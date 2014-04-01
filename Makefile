RUSTC ?= rustc
RUSTFLAGS ?=

src_files=\
	src/rustiles.rs \
	src/tile.rs \
	src/workqueue.rs

all: build/rustiles

build/rustiles: $(src_files)
	mkdir -p build
	$(RUSTC) $(RUSTFLAGS) -o build/rustiles src/rustiles.rs

build/testsuite: $(src_files)
	mkdir -p build
	$(RUSTC) $(RUSTFLAGS) -A dead_code --test -o build/testsuite src/rustiles.rs

check: build/testsuite
	RUSTILES_TEST_FIXTURES=`pwd`/fixtures ./build/testsuite

bench: build/testsuite
	RUSTILES_TEST_FIXTURES=`pwd`/fixtures ./build/testsuite --bench

clean:
	rm -rf build

.PHONY: all check clean
