build:
	wasm-pack build --dev --target nodejs

release:
	wasm-pack build --release --target nodejs
	wasm-opt -O3 pkg/fast_match_bg.wasm -o pkg/fast_match_bg.wasm

profiling:
	wasm-pack build --profiling --target nodejs
