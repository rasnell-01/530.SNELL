Develop a Python program that acts as a web client capable of sending HTTP requests to a web server, retrieving information, and processing the responses.

Requirements:

Use the requests library:
Your client should be able to send at least two types of HTTP requests (e.g., GET and POST).
Handle responses:
For GET requests, display the retrieved data (e.g., JSON response) in a readable format.
For POST requests, send some sample data to the server, then display the server’s response.
Error handling:
Implement basic error handling for HTTP response codes (e.g., display a message for 404 Not Found, 500 Internal Server Error, etc.).
Example Workflow:

Step 1: Make a GET request
The program should make a GET request to a public API, such as https://jsonplaceholder.typicode.com/posts, and display the data returned (e.g., titles of the posts).

Step 2: Make a POST request
Your client should also send a POST request to a service like https://jsonplaceholder.typicode.com/posts, passing some mock data (e.g., title, body, userId). After receiving the response, the program should display whether the request was successful and print the response content.

Step 3: Error Handling
The client should check if the request failed (e.g., due to a bad URL or network issue) and handle these cases gracefully, informing the user.

Deliverables: - A Python script (web_client.py) that implements the web client with the required functionality, placed into the Assignment 05 folder in your GitHub repository.
