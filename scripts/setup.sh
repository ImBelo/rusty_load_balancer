#!/bin/bash

# Funzione di cleanup
cleanup() {
    echo " Stopping all servers..."
    pkill -f "python3 -m http.server" 2>/dev/null || true
    pkill -f "load-balancer-rs" 2>/dev/null || true
    exit 0
}

# Registra il trap per CTRL+C e terminazione
trap cleanup SIGINT SIGTERM

echo "Starting backend servers..."

pkill -f "python3 -m http.server" 2>/dev/null || true
sleep 2

mkdir -p /tmp/backend1 /tmp/backend2 /tmp/backend3

echo "<h1>Backend 1 - Server 8081</h1>" > /tmp/backend1/index.html
echo "<h1>Backend 2 - Server 8082</h1>" > /tmp/backend2/index.html  
echo "<h1>Backend 3 - Server 8083</h1>" > /tmp/backend3/index.html

# Facciamo partire i server
python3 -m http.server 8081 --directory /tmp/backend1 &
SERVER1_PID=$!

python3 -m http.server 8082 --directory /tmp/backend2 &
SERVER2_PID=$!

python3 -m http.server 8083 --directory /tmp/backend3 &
SERVER3_PID=$!

echo "Starting Load Balancer..."
cargo run &
LB_PID=$!

echo "All services started. Press Ctrl+C to stop everything."

wait
