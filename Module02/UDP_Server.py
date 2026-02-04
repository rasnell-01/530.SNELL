import socket, threading, time, random
from datetime import datetime
from typing import Dict, Tuple

SERVER_PORT = 9999
SERVER_HOST = '0.0.0.0'
BUFFER_SIZE = 1024
LOG_FILE = 'server_log.txt'

MINIMUM_INTERVAL = 5
MAXIMUM_INTERVAL = 30

class UDP_Server:
    def __init__(self, server_host: str, server_port: int, buffer_size: int):
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.server_host = server_host
        self.server_port = server_port
        self.buffer_size = buffer_size
        self.socket.bind((self.server_host, self.server_port))
        self.clients: Dict[int, Tuple[str, int]] = {}
        self.clients_lock = threading.Lock()
        self.running = True
        print(f"[SERVER] UDP Server running at {self.server_host}:{self.server_port}")
        self.log_message(f"Server started at {self.server_host}:{self.server_port}")

    def log_message(self, message: str):
        timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        log_entry = f"[{timestamp}] {message}\n"
        with open(LOG_FILE, 'a') as file:
            file.write(log_entry)

    def handle_client_message(self):
        print(f"[SERVER] Listening for message from client...")

        while self.running:
            try:
                self.socket.settimeout(1.0)
                data, address = self.socket.recvfrom(BUFFER_SIZE)
                message = data.decode('ASCII').strip()

                print(f"[SERVER] Received message: {message} from {address}")

                parsed_message = message.split()
                if len(parsed_message) != 2:
                    print(f"[SERVER] Invalid message format: {message}")
                    continue
                command = parsed_message[0].upper()
                try:
                    client_id = int(parsed_message[1])
                except ValueError:
                    print(f"[SERVER] Invalid Client ID: {parsed_message[1]}")
                    continue

                if command == 'R':
                    with self.clients_lock:
                        self.clients[client_id] = address
                    print(f"[SERVER] Client registered at {address} with ID {client_id}")
                    self.log_message(f"Client registered at {address} with ID {client_id}")

                elif command == 'U':
                    with self.clients_lock:
                        if client_id in self.clients:
                            del self.clients[client_id]
                            print(f"[SERVER] Client unregistered at {address} with ID {client_id}")
                            self.log_message(f"Client unregistered at {address} with ID {client_id}")
                        else:
                            print(f"[SERVER] Client {client_id} not found for unregistration")

                else:
                    print(f"[SERVER] Invalid command: {command}")

            except socket.timeout:
                continue
            except Exception as e:
                if self.running:
                    print(f"[SERVER] Exception message: {e}")

    def send_periodic_message(self):
        print("[SERVER] Starting periodic message sender...")

        while self.running:
            interval = random.randint(MINIMUM_INTERVAL, MAXIMUM_INTERVAL)
            print(f"[SERVER] Sending periodic message in {interval} seconds...")

            time.sleep(interval)

            if not self.running:
                break

            timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S.%f')
            message = f"SERVER_MESSAGE: {timestamp}"

            with self.clients_lock:
                if not self.clients:
                    print(f"[SERVER] No clients registered")
                    continue
                print(f"[SERVER] Sending message to {len(self.clients)} client(s): {message}")
                self.log_message(f"Sending message to {len(self.clients)} client(s): {message}")

                for client_id, address in self.clients.items():
                    try:
                        self.socket.sendto(message.encode('ASCII'), address)
                        print(f"[SERVER] => Sent to client {client_id} at {address}")
                    except Exception as e:
                        print(f"[SERVER] Exception while sending to client {client_id}: {e}")
                        self.log_message(f"Exception while sending to client {client_id}: {e}")

    def run(self):
        listener_thread = threading.Thread(target=self.handle_client_message, daemon=True)
        listener_thread.start()

        sender_thread = threading.Thread(target=self.send_periodic_message, daemon=True)
        sender_thread.start()

        try:
            while self.running:
                time.sleep(1)

        except KeyboardInterrupt:
            print("\n[SERVER] Stopping server...")
            self.running = False
            listener_thread.join(timeout=5)
            sender_thread.join(timeout=5)
            self.socket.close()
            print(f"[SERVER] Server stopped")

def main():
    server = UDP_Server(SERVER_HOST, SERVER_PORT, BUFFER_SIZE)
    server.run()

if __name__ == '__main__':
    main()