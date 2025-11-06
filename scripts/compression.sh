#!/bin/bash

echo richiesta con Accept-Encoding: identity Header
echo "$(curl -s -H "Accept-Encoding: identity" http://localhost:3000/ | wc -c) bytes"

echo richiesta con Accept-Encoding: gzip Header
echo "$(curl -s -H "Accept-Encoding: gzip" http://localhost:3000/ | wc -c) bytes"

echo richiesta senza Header
echo "$(curl -s http://localhost:3000/ | wc -c) bytes"
