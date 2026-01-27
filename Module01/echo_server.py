#pulled from module01PartE.html from CS530 course
# echo-server.py

import socket
from dotenv import load_dotenv
import os

load_dotenv()

HOST = os.getenv("ECHO_SERVER_HOST")  # Standard loopback interface address (localhost)
PORT = int(os.getenv("ECHO_SERVER_PORT"))  # Port to listen on (non-privileged ports are > 1023)

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as echo_server:
    echo_server.bind((HOST, PORT))
    echo_server.listen()
    conn, addr = echo_server.accept()
    with conn:
        print(f"Connected by {addr}")
        while True:
            data = conn.recv(1024)
            if not data:
                break
            conn.sendall(data)