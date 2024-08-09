RFLAGS="-C link-arg=-s"

build-staker:
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p near-staker --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/near_staker.wasm ./res/near_staker.wasm

test-staker: build-staker
	mkdir -p target/near/near_staker
	RUSTFLAGS=$(RFLAGS) RUST_TEST_THREADS=1 cargo test -p near-staker

build: build-staker
test: test-staker

clean:
	cargo clean
	rm -rf res/
