import socket, time, random, sys
from datetime import datetime

SERVER_HOST = '127.0.0.1'
SERVER_PORT = 9999
BUFFER_SIZE = 1024

MINIMUM_RUNTIME = 15
MAXIMUM_RUNTIME = 90

INITIAL_TIMEOUT = 0.5
MAXIMUM_TIMEOUT = 5.0
BACKOFF_MULTIPLIER = 1.5

class UDP_Client:
    def __init__(self, client_id: int, server_host: str, server_port: int):
        self.client_id = client_id
        self.server_host = server_host
        self.server_port = server_port
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.socket.bind(('',0))
        self.log_file = f'client_{self.client_id}_log.txt'
        self.running = True
        print(f"[CLIENT {self.client_id}] Started on port {self.socket.getsockname()[1]}")
        self.log_to_file(f"Client {self.client_id} started on port {self.socket.getsockname()[1]}")

    def log_to_file(self, message: str):
        timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S.%f')
        log_entry = f"[{timestamp}] [{message}]\n"
        with open(self.log_file, 'a') as file:
            file.write(log_entry)

    def register(self):
        message = f"R {self.client_id}"
        try:
            self.socket.sendto(message.encode('ASCII'), (self.server_host, self.server_port))
            print(f"[CLIENT {self.client_id}] Sent registration to server: {message}")
            self.log_to_file(f"Sent registration to server: {message}")
        except Exception as e:
            print(f"[CLIENT {self.client_id}] Failed to send registration to server: {e}")
            self.log_to_file(f"Failed to send registration to server: {e}")

    def unregister(self):
        message = f"R {self.client_id}"
        try:
            self.socket.sendto(message.encode('ASCII'), (self.server_host, self.server_port))
            print(f"[CLIENT {self.client_id}] Sent unregistration to server: {message}")
            self.log_to_file(f"Sent unregistration to server: {message}")
        except Exception as e:
            print(f"Failed to send unregistration to server: {e}")
            self.log_to_file(f"Failed to send unregistration to server: {e}")

    def listen_for_message(self, duration: int):
        print(f"[CLIENT {self.client_id}] Listening for messages for {duration} seconds...")
        start_time = time.time()
        message_count = 0

        while self.running:
            try:
                self.socket.settimeout(self.timeout)
                data, address = self.socket.recvfrom(BUFFER_SIZE)
                message = data.decode('ASCII')
                message_count += 1
                print(f"[CLIENT {self.client_id}] Received message #{message_count}: {message}")
                self.log_to_file(f"Received message #{message_count} from server: {message}")
                self.timeout = INITIAL_TIMEOUT

            except socket.timeout:
                self.timeout = min(self.timeout * BACKOFF_MULTIPLIER, MAXIMUM_TIMEOUT)
                continue

            except Exception as e:
                print(f"[CLIENT {self.client_id}] Error receiving message: {e}")
                self.log_to_file(f"Error receiving message: {e}")

        print(f"[CLIENT {self.client_id}] Finished listening for messages. Received {message_count} messages from server.")
        self.log_to_file(f"Finished listening for messages: {message_count} messages from server.")

    def run(self):
        try:
            self.register()
            time.sleep(0.5)
            runtime = random.randint(MINIMUM_RUNTIME, MAXIMUM_RUNTIME)
            print(f"[CLIENT {self.client_id}] Started for {runtime} seconds...")
            self.log_to_file(f"Started for {runtime} seconds...")
            self.unregister()

        except KeyboardInterrupt:
            print(f"\n[CLIENT {self.client_id}] Received user interrupt.")
            self.log_to_file(f"Received user interrupt.")
            self.unregister()

        finally:
            self.running = False
            self.socket.close()
            print(f"[CLIENT {self.client_id}] Client terminated.")
            self.log_to_file(f"Client terminated.")

def main():
    if len(sys.argv) > 1:
        try:
            client_id = int(sys.argv[1])
        except ValueError:
            print("UDP_Client.py [client_id]")
            print("Using random client ID instead between 1 and 9999.")
            client_id = random.randint(1, 9999)

    else:
        client_id = random.randint(1, 9999)

    print(f"Starting client with ID: {client_id}")
    client = UDP_Client(client_id, SERVER_HOST, SERVER_PORT)
    client.run()

if __name__ == "__main__":
    main()