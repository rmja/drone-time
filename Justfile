features := 'systick stm32f4'

# Install dependencies
deps:
	type drone >/dev/null || cargo install drone

# Reformat the source code
fmt:
	cargo fmt

# Check the source code for mistakes
lint:
	cargo clippy --features "{{features}}"

# Build the documentation
doc:
	cargo doc --features "{{features}}"

# Open the documentation in a browser
doc-open: doc
	cargo doc --features "{{features}}" --open

# Run the tests
test:
	cargo test --features "{{features}} std" \
		--target=$(rustc --version --verbose | sed -n '/host/{s/.*: //;p}')

# Update README.md
readme:
	cargo readme -o README.md