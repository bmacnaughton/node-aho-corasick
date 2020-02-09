build:
	wasm-pack build --dev --target nodejs

release:
	wasm-pack build --release --target nodejs

profiling:
	wasm-pack build --profiling --target nodejs