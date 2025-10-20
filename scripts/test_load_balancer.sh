#!/bin/bash
echo "Quick Load Balancer Test"
echo "=========================="

# Test singola richiesta
echo "1. Single request:"
curl -s -w "Time: %{time_total}s | HTTP: %{http_code}\n" http://localhost:3000/

echo

sleep 1
echo "Making 10 requests..."
for i in {1..10}; do
    curl http://localhost:3000/ &
done
wait

wait

