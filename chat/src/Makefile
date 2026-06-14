CC = gcc
CFLAGS = -g -Wall -D_POSIX_C_SOURCE=200809L -I../include -O2

all: chat-server chat-client

chat-server: server.c protocol.c crypto.c utils.c logging.c $(OBJDIR:=1)
	gcc $(CFLAGS) -o $@ server.c protocol.c crypto.c utils.c logging.c

chat-client: client.c protocol.c crypto.c utils.c logging.c
	gcc $(CFLAGS) -o $@ client.c protocol.c crypto.c utils.c logging.c


clean:
	rm -f chat-server chat-client *.o

test: test_server.sh test_client.sh
	./test_server.sh 5555 keys/server_whitelist &
	sleep 1
	./test_client.sh keys/client_A.pub keys/client_A.pem

.PHONY: all clean test install
