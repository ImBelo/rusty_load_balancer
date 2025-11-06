#!/bin/bash
LOG_DIR="./logs"
mkdir -p $LOG_DIR

# Funzione di cleanup
cleanup() {
    echo " Stopping all servers..."
    killall python3 2>/dev/null || true
    pkill -f "load-balancer-rs" 2>/dev/null 
    rm -rf logs/
    exit 0
}

# Registra il trap per CTRL+C e terminazione
trap cleanup SIGINT SIGTERM

echo "Starting backend servers..."
killall python3 2>/dev/null || true
sleep 2

python3 -m server 8081 > "$LOG_DIR/backend_8081.log" 2>&1 &

python3 -m server 8082 > "$LOG_DIR/backend_8082.log" 2>&1 &

python3 -m server 8083 > "$LOG_DIR/backend_8083.log" 2>&1 & 

sleep 2

echo "Starting Load Balancer..."
cargo run --release &

echo "All services started. Press Ctrl+C to stop everything."

wait
