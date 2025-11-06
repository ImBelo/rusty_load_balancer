#!/usr/bin/env python3
import concurrent.futures
import requests
import time
from collections import defaultdict

def make_request(_):
    start = time.time()
    try:
        response = requests.get("http://127.0.0.1:3000/", timeout=10)
        duration = time.time() - start
        
        if "8081" in response.text:
            return ("Backend 1", duration)
        elif "8082" in response.text:
            return ("Backend 2", duration) 
        elif "8083" in response.text:
            return ("Backend 3", duration)
        else:
            return ("Unknown", duration)
    except:
        return ("Error", time.time() - start)

if __name__ == "__main__":
    print("Starting load test with 1000 requests...")
    
    results = defaultdict(list)
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=20) as executor:
        for backend, duration in executor.map(make_request, range(1000)):
            results[backend].append(duration)
    
    print("=== LOAD BALANCER RESULTS ===")
    total = sum(len(v) for v in results.values())
    print(f"Total requests: {total}")
    
    for backend, durations in results.items():
        if durations:
            count = len(durations)
            avg = sum(durations) / count
            max_time = max(durations)
            percentage = (count / total) * 100
            print(f"{backend}: {count:3d} requests ({percentage:.1f}%) | Avg: {avg:.3f}s | Max: {max_time:.3f}s")
