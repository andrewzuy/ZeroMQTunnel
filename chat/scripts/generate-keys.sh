#!/bin/bash
openssl genpkey -algorithm RSA -out priv.pem -pkeyopt rsa_keygen_bits:2048 2>/dev/null || exit 1
openssl pkey -in priv.pem -pubout -out pub.pem 2>/dev/null || exit 1
echo "Keys ready:" && ls -lh keys/*.pem 2>/dev/null | awk '{print "  "$5"  "$NF}'
