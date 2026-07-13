# Build dx-serializer and connect to `dx serializer` CLI
BIN := "G:/Dx/bin"

# Build release with 12 jobs
build:
    cargo build --release -j 12
    @echo "Build complete: target/release/dx-serializer.exe"

# Publish to workspace bin (connects `dx serializer` CLI)
publish: build
    cp target/release/dx-serializer.exe {{BIN}}/dx-serializer.exe
    cp target/release/dx-serializer.exe {{BIN}}/dx-serialize.exe
    @echo "Published to {{BIN}}/"

# Build + publish
pub: publish

# Build + publish + verify
all: publish
    {{BIN}}/dx-serializer.exe --version
    dx serializer dx --help 2>&1 | head -3
