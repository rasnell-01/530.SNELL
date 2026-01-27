import socket
from time import perf_counter_ns
from dotenv import load_dotenv
import os

load_dotenv()

SERVER_HOST = os.getenv("ECHO_SERVER_HOST")
SERVER_PORT = int(os.getenv("ECHO_SERVER_PORT"))
MESSAGE = b'PING!!!'
TRIALS = int(os.getenv("NUMBER_OF_TRIALS"))

time_milliseconds = []

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as client:
    client.connect((SERVER_HOST, SERVER_PORT))

    for i in range(TRIALS):
        start_time = perf_counter_ns()
        client.sendall(MESSAGE)
        response = client.recv(1024)
        end_time = perf_counter_ns()

        duration_milliseconds = (end_time - start_time) / 1_000_000
        time_milliseconds.append(duration_milliseconds)

        print(f'Trial {i+1}: {duration_milliseconds:.3f} ms')

    average_time = sum(time_milliseconds) / len(time_milliseconds)

    print("\n--- RESULTS ---")
    print(f"Average run time over {TRIALS} trials: {average_time:.3f} ms")