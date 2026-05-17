.PHONY: all build test clean install

all: build test

build:
	cargo build --release

test:
	cargo test

test-integration:
	cargo test gtp_client -- --nocapture --test-threads=1

test-unit:
	cargo test -- --skip gtp_client

clean:
	cargo clean

install: build
	@mkdir -p ../KataGoGo/SharedCore
	cp target/release/libgo_core.a ../KataGoGo/SharedCore/
	cp src/go_core.h ../KataGoGo/SharedCore/
	@echo "Installed libgo_core.a and go_core.h to Xcode project"
