#!/bin/bash
set -e
cd "$(dirname "$0")/../.." || exit 1
KEYGEN_PATH="./chat/keygen"
"$KEYGEN_PATH" priv.pem pub.pem && \
mkdir -p keys && \
"$KEYGEN_PATH" keys/priv.pem keys/pub.pem && \
echo "✅ Test keys generated in chat/keys/"

