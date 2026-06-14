# ZeroMQTunnel Build System - Phase 2-7+
# Clean build system with dependency tracking

CC := gcc
CFLAGS := -g -O0 -Wall -D_POSIX_C_SOURCE=200809L -I../include
COMMON_FLAGS := $(CFLAGS) -Wl,-z,defs  # Ensure symbol visibility for tests
LDFLAGS := -lssl -lcrypto
DEPRECATED_FLAGS := -Wno-deprecated  # Phase 4: Suppress EVP warnings (will be removed when fully migrated)

# Source files for server/client binary combination
SRCS_SERVER_CLIENT := \
    ../include/service.c \
    ../include/server.c \
    ../include/config.c \
    ../include/protocol.c \
    ../include/crypto.c \
    ../include/utils.c \
    ../include/logging.c


# Source files for keygen utility
SRCS_KEYGEN := tools/keygen.c ../include/crypto.h ../include/utils.h

.PHONY: all clean test help rebuild

all: chat-server
	@echo "✅ Phase 4 EVP crypto implementation ready"

chat-server: $(SRCS_SERVER_CLIENT)
	$(CC) -g -O2 \
		-I../include \
		-Wno-deprecated \
		src/server.c src/client.c src/protocol.c src/crypto.c src/utils.c src/logging.c \
		-lssl -lcrypto \
		-o chat-server

keygen: $(SRCS_KEYGEN) tools/keygen.c
	$(CC) -g -O2 \
		-Wno-deprecated \
		tools/keygen.c src/crypto.c \
		-lssl -lcrypto \
		-o keygen

test: all
	@echo "Testing Phase 4 EVP integration..."
	./chat-server 5559 & sleep 1 && kill %1 2>/dev/null
	./keygen keys/priv.pem keys/pub.pem 2>&1 | head -3

clean:
	rm -f *.o chat-server keygen
	rm -rf src/*.o build/*

help:
	@echo "Usage:"
	@echo "  make all        - Build chat-server with Phase 4 EVP crypto"
	@echo "  make clean      - Remove binaries and object files"
	@echo "  make test       - Quick smoke test of build"

rebuild: clean all

.PHONY: all clean test help rebuild

