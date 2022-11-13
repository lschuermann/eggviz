all: web

web:
	wasm-pack build --target web --release
run: web
	live-server --browser=google-chrome

