#!/bin/bash

echo "Starting backend servers on ports 8081, 8082, 8083..."

# Start Python HTTP servers for each backend
python3 -m http.server 8081 --directory /app/backend1 &
python3 -m http.server 8082 --directory /app/backend2 &
python3 -m http.server 8083 --directory /app/backend3 &

echo "Backend servers started."
echo "Backend 1: http://localhost:8081"
echo "Backend 2: http://localhost:8082" 
echo "Backend 3: http://localhost:8083"

# Wait for all background processes
wait
