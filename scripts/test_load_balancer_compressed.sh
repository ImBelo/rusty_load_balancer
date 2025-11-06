#!/bin/bash

echo "Starting load test with 1000 requests..."
# Crea file temporaneo per risultati
RESULTS_FILE=$(mktemp)

seq 1 1000 | xargs -I {} -P 20 bash -c '
    # Fai la richiesta e prendi tutto - RIMUOVI gzip e aggiungi --compressed
    full_response=$(curl -s --compressed -w "|TIME:%{time_total}\n" "http://127.0.0.1:3000/")
    
    # Estrai il tempo
    time=$(echo "$full_response" | grep "|TIME:" | cut -d: -f2)
    
    # Controlla quale backend ha risposto guardando il contenuto
    if echo "$full_response" | grep -q "8081"; then
        echo "Backend 1|$time"
    elif echo "$full_response" | grep -q "8082"; then
        echo "Backend 2|$time" 
    elif echo "$full_response" | grep -q "8083"; then
        echo "Backend 3|$time"
    else
        echo "Unknown|$time"
    fi
' > "$RESULTS_FILE"

# Analizza i risultati
awk -F'|' '
/Backend 1/ { count1++; time1 += $2; if($2 > max1) max1 = $2 }
/Backend 2/ { count2++; time2 += $2; if($2 > max2) max2 = $2 }
/Backend 3/ { count3++; time3 += $2; if($2 > max3) max3 = $2 }
END {
    total = count1 + count2 + count3;
    printf "=== LOAD BALANCER RESULTS ===\n";
    printf "Total requests: %d\n", total;
    printf "Backend 1: %3d requests (%.1f%%) | Avg: %.3fs | Max: %.3fs\n", count1, (count1/total)*100, (count1 ? time1/count1 : 0), max1;
    printf "Backend 2: %3d requests (%.1f%%) | Avg: %.3fs | Max: %.3fs\n", count2, (count2/total)*100, (count2 ? time2/count2 : 0), max2;
    printf "Backend 3: %3d requests (%.1f%%) | Avg: %.3fs | Max: %.3fs\n", count3, (count3/total)*100, (count3 ? time3/count3 : 0), max3;
}' "$RESULTS_FILE"

rm "$RESULTS_FILE"
