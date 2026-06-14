# ZeroMQTunnel Build System
# Targets: chat-server, chat-client, keygen

CC = gcc
CFLAGS = -g -O2 -Wall -D_POSIX_C_SOURCE=200809L -Wextra -I../include
LDFLAGS = -lssl -lcrypto
LIBSSL_FLAGS = -Wno-deprecated  # OpenSSL 3.0+ deprecation warnings

# Source files (shared by server, client, keygen)
COMMON_SRCS = \
    ../include/server.c \
    ../include/client.c \
    ../include/protocol.c \
    ../include/crypto.c \
    ../include/utils.c \
    ../include/logging.c

all: chat-server chat-client keygen

chat-server: server.c $(COMMON_SRCS)
	$(CC) $(CFLAGS) -o $@ \
		$< server.c \
		$(COMMON_SRCS:%=%:.common_) \
		$(LDFLAGS) $(LIBSSL_FLAGS)

server.o: server.c ../include/server.h ../include/protocol.h
	$(CC) $(CFLAGS) -c $< -o $@

client.o: client.c ../include/client.h ../include/protocol.h
	$(CC) $(CFLAGS) -c $< -o $@

# Simpler makefile for testing
.PHONY: all clean test

clean:
	rm -f *.o chat-server chat-client keygen

test: all
	./keygen priv.pem pub.pem 2>/dev/null && echo "Keygen OK" || echo "Keygen FAILED"
	./chat-server 5555 & sleep 2 && kill %1 2>/dev/null
	./chat-client priv.pem pub.pem 2>/dev/null && echo "Client OK" || echo "Client FAILED"

.PHONY: all clean test
