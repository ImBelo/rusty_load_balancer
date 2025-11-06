from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn
import json
import sys
import os

class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    """Gestisce ogni richiesta in un thread separato"""
    daemon_threads = True

class HeaderInspectorHandler(BaseHTTPRequestHandler):
    def do_HEAD(self):
        port = os.environ.get('SERVER_PORT', 'unknown')
        
        print("\n" + "="*50, flush=True)
        print(f"HEAD REQUEST RICEVUTA", flush=True)
        print("="*50, flush=True)
        print(f"SERVER PORT: {port}", flush=True)
        print(f"PATH: {self.path}", flush=True)
        print(f"METODO: {self.command}", flush=True)
        print(f"CLIENT: {self.client_address}", flush=True)
        print("\nHEADERS:", flush=True)
        for header, value in self.headers.items():
            print(f"  {header}: {value}", flush=True)
        print("="*50, flush=True)
        
        # Invia solo gli header, senza body
        self.send_response(200)
        self.send_header('Content-type', 'text/html; charset=utf-8')
        self.send_header('X-Backend-Server', f'Python-Custom-{port}')
        self.end_headers()
    
    def do_GET(self):
        port = os.environ.get('SERVER_PORT', 'unknown')
        
        # FLUSH immediato dell'output
        print("\n" + "="*50, flush=True)
        print(f"NUOVA RICHIESTA RICEVUTA", flush=True)
        print("="*50, flush=True)
        print(f"SERVER PORT: {port}", flush=True)
        print(f"PATH: {self.path}", flush=True)
        print(f"METODO: {self.command}", flush=True)
        print(f"CLIENT: {self.client_address}", flush=True)
        print("\nHEADERS:", flush=True)
        for header, value in self.headers.items():
            print(f"  {header}: {value}", flush=True)
        print("="*50, flush=True)
        
        html_content = f"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Backend Server - Port {port}</title>
            <style>
                body {{ font-family: Arial, sans-serif; margin: 40px; }}
                .header {{ background: #f0f0f0; padding: 10px; margin: 5px 0; border-left: 4px solid #007acc; }}
                .port {{ color: blue; font-weight: bold; }}
                .info {{ background: #e0f7fa; padding: 15px; border-radius: 5px; }}
                .client {{ color: green; }}
            </style>
        </head>
        <body>
            <div class="info">
                <h1>Backend Server</h1>
                <h2>Port: <span class="port">{port}</span></h2>
                <p>Client: <span class="client">{self.client_address}</span></p>
            </div>
            
            <h3>Headers Ricevuti:</h3>
            <div id="headers">
        """
        
        for header, value in self.headers.items():
            html_content += f'<div class="header"><strong>{header}:</strong> {value}</div>'
        
        html_content += """
            </div>
            <hr>
            <p><em>Controlla la console del server per vedere i log completi</em></p>
        </body>
        </html>
        """
        
        # Invia la risposta
        self.send_response(200)
        self.send_header('Content-type', 'text/html; charset=utf-8')
        self.send_header('X-Backend-Server', f'Python-Custom-{port}')
        self.end_headers()
        self.wfile.write(html_content.encode('utf-8'))
    
    def do_POST(self):
        port = os.environ.get('SERVER_PORT', 'unknown')
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length) if content_length > 0 else b''
        
        print("\n" + "="*50, flush=True)
        print(f"RICHIESTA POST RICEVUTA", flush=True)
        print("="*50, flush=True)
        print(f"SERVER PORT: {port}", flush=True)
        print(f"PATH: {self.path}", flush=True)
        print(f"CLIENT: {self.client_address}", flush=True)
        print("\nHEADERS:", flush=True)
        for header, value in self.headers.items():
            print(f" {header}: {value}", flush=True)
        print(f"\nBODY: {post_data.decode('utf-8', errors='ignore')}", flush=True)
        print("="*50, flush=True)
        
        response = {
            "status": "received",
            "method": "POST", 
            "path": self.path,
            "server_port": port,
            "client_address": str(self.client_address),
            "content_length": content_length,
            "headers": dict(self.headers)
        }
        
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(response, indent=2).encode('utf-8'))
    
    def log_message(self, format, *args):
        return

def run_server(port=8081):
    # Imposta la porta come variabile d'ambiente
    os.environ['SERVER_PORT'] = str(port)
    
    server_address = ('', port)
    httpd = ThreadedHTTPServer(server_address, HeaderInspectorHandler)
    print(f"Server header inspector (THREADED) in esecuzione sulla porta {port}")
    print(f"Usa: curl http://localhost:{port}")
    print(f"Oppure apri: http://localhost:{port} nel browser") 
    print("Premi Ctrl+C per fermare il server\n")
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print(f"\nServer sulla porta {port} fermato")

if __name__ == '__main__':
    if len(sys.argv) > 1:
        port = int(sys.argv[1])
    else:
        port = 8081
    
    run_server(port)
