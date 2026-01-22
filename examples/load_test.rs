use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use reqwest::Client;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let url = "http://127.0.0.1:3000/"; // L'URL del tuo Load Balancer
    let total_requests = 1000;
    let concurrency = 50; 
    
    let client = Client::builder()
        .pool_max_idle_per_host(concurrency)
        .build()
        .unwrap();

    let stats = Arc::new(Mutex::new(HashMap::new()));
    let start_test = Instant::now();

    println!("🚀 Avvio test di carico: {} richieste totali (concorrenza: {})...", total_requests, concurrency);

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut tasks = vec![];

    for _ in 0..total_requests {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let stats = stats.clone();

        tasks.push(tokio::spawn(async move {
            let start_req = Instant::now();
            let res = client.get(url).send().await;
            
            let duration = start_req.elapsed();
            
            if let Ok(response) = res {
                let body = response.text().await.unwrap_or_default();
                
                let backend = if body.contains("8081") { "Backend 1" }
                             else if body.contains("8082") { "Backend 2" }
                             else if body.contains("8083") { "Backend 3" }
                             else { "Unknown" };

                let mut s = stats.lock().unwrap();
                let entry = s.entry(backend.to_string()).or_insert((0, Duration::ZERO));
                entry.0 += 1; // Incrementa contatore
                entry.1 += duration; // Somma tempo
            }
            drop(permit);
        }));
    }

    futures::future::join_all(tasks).await;

    let total_duration = start_test.elapsed();
    let final_stats = stats.lock().unwrap();

    println!("📊 RISULTATI LOAD BALANCER");
    println!("Tempo Totale:  {:.2?}", total_duration);
    println!("Richieste/sec: {:.2}", total_requests as f64 / total_duration.as_secs_f64());

    for (name, (count, total_time)) in final_stats.iter() {
        let percent = (*count as f64 / total_requests as f64) * 100.0;
        let avg = total_time.as_secs_f64() / (*count as f64);
        println!("{}: {:>4} rich. ({:>5.1}%) | Media: {:.4}s", name, count, percent, avg);
    }
}
