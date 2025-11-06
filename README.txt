Linux:
To run: ./scripts/setup.sh
Test: ./scripts/test_load_balancer.sh
Single request: curl http://localhost:3000

Docker:
To run: docker-compose up
Go inside: docker exec -it load_balancer-load-balancer-1 bash
Test: ./scripts/test_load_balancer.sh
Single request: curl http://localhost:3000 // MUST BE INSIDE DOCKER EXEC BECAUSE OF SANDBOX
