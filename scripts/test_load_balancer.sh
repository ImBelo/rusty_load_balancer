#!/bin/bash
seq 1 1000 | xargs -I {} -P 50 bash -c '
    response=$(curl -s -w "TIME:%{time_total}" "http://127.0.0.1:3000/")
    backend=$(echo "$response" | grep "Backend" | head -1)
    time=$(echo "$response" | grep "TIME:" | cut -d: -f2)
    echo "$backend|$time"
' | awk -F'|' '
/Backend 1/ { count1++; time1 += $2; if($2 > max1) max1 = $2 }
/Backend 2/ { count2++; time2 += $2; if($2 > max2) max2 = $2 }
/Backend 3/ { count3++; time3 += $2; if($2 > max3) max3 = $2 }
END {
    total = count1 + count2 + count3;
    printf "=== LOAD BALANCER RESULTS ===\n";
    printf "Backend 1: %3d requests | Avg: %.3fs | Max: %.3fs\n", count1, (count1 ? time1/count1 : 0), max1;
    printf "Backend 2: %3d requests | Avg: %.3fs | Max: %.3fs\n", count2, (count2 ? time2/count2 : 0), max2;
    printf "Backend 3: %3d requests | Avg: %.3fs | Max: %.3fs\n", count3, (count3 ? time3/count3 : 0), max3;
}'
