# ==============================================================================
# ZeroMQTunnel - Simple Build System (Phase 2-7)
# This Makefile compiles chat-server, chat-client, and keygen utilities
# ==============================================================================

.PHONY: all clean test help

# Compiler and flags
CC := gcc
CFLAGS := -g -O0 -Wall -D_GNU_SOURCE -I$(CURDIR)/include
LDFLAGS := -lssl -lcrypto
DEPRECATED_FLAGS := -Wno-deprecated  # Suppress OpenSSL 3.0+ deprecation warnings

# Source files shared by all targets
SRCS_SERVER := $(wildcard src/*.c)
SRCS_KEYGEN := tools/keygen.c src/crypto.c

# ==============================================================================
# All targets
# ==============================================================================
all: chat-server chat-client keygen
	@echo "✅ Build complete! Binaries:"
	@ls -lh chat-server chat-client keygen | awk '{print "  "$5"  "$NF}'

# ==============================================================================
# Compile chat-server (server + client source files combined)
# ==============================================================================
chat-server: $(SRCS_SERVER)
	@echo "Compiling chat-server..."
	$(CC) $(CFLAGS) -o $@ \
		src/server.c \
		src/client.c \
		src/protocol.c \
		src/crypto.c \
		src/utils.c \
		src/logging.c \
		tools/keygen.c \
		$(LDFLAGS)

# ==============================================================================
# Compile chat-client (client + server source files combined)
# ==============================================================================
chat-client: $(SRCS_SERVER)
	@echo "Compiling chat-client..."
	$(CC) $(CFLAGS) -o $@ \
		src/server.c \
		src/client.c \
		src/protocol.c \
		src/crypto.c \
		src/utils.c \
		src/logging.c \
		tools/keygen.c \
		$(LDFLAGS)

# ==============================================================================
# Compile keygen utility
# ==============================================================================
keygen: $(SRCS_KEYGEN)
	@echo "Compiling keygen..."
	$(CC) $(CFLAGS) -o $@ \
		$(SRCS_KEYGEN) \
		$(LDFLAGS)

# ==============================================================================
# Clean built files
# ==============================================================================
clean:
	rm -f chat-server chat-client keygen server.log keys/priv*.pem keys/pub*.pem testpriv*.pem testpub*.pem

# ==============================================================================
# Regenerate OpenSSL key pair
# ==============================================================================
keys:
	@echo "Generating RSA-2048 keypair..."
	mkdir -p keys
	openssl genpkey -algorithm RSA -out keys/priv.pem -pkeyopt rsa_keygen_bits:2048 2>/dev/null || exit 1
	openssl pkey -in keys/priv.pem -pubout -out keys/pub.pem 2>/dev/null || exit 1
	@echo "✅ Keys generated:"
	@ls -lh keys/*.pem

# ==============================================================================
# Run a quick test: generate keys and start server
# ==============================================================================
test: all keys
	@./keygen priv.pem pub.pem 2>&1 | grep -v "^Generating\|^Context" || true
	@echo ""
	@echo "Starting server on port 5558..."
	@./chat-server 5558 &
	@sleep 1
	@echo "✅ Server started (Ctrl+C to stop)"

# ==============================================================================
# Show this help message
# ==============================================================================
help:
	@echo "ZeroMQTunnel Build System"
	@echo ""
	@echo "Usage:"
	@echo "  make all           - Build chat-server, chat-client, keygen"
	@echo "  make clean         - Remove built files"
	@echo "  make keys          - Generate RSA-2048 key pair (keys/*.pem)"
	@echo "  make test          - Build everything and start server on port 5558"
	@echo "  make help          - Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  ./chat-server [port]      - Start relay server"
	@echo "  ./chat-client priv.pem pub.pem  - Connect to local server"
	@echo "  Client commands:"
	@echo "    /quit                    - Exit client"
	@echo "    /to <recipient>          - Set message recipient"

# ==============================================================================
# Debug rebuild (show all commands)
# ==============================================================================
rebuild: clean all help

