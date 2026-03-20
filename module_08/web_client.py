import requests
import json

BASE_URL = "https://jsonplaceholder.typicode.com"

DIVIDER = "─" * 60

def print_section(title: str) -> None:
    print(f"\n{DIVIDER}")
    print(f"  {title}")
    print(DIVIDER)

def handle_http_error(response: requests.Response) -> None:
    code = response.status_code
    messages = {
        400: "400 Bad Request – The server could not understand the request.",
        401: "401 Unauthorized – Authentication is required.",
        403: "403 Forbidden – You do not have permission to access this resource.",
        404: "404 Not Found – The requested resource does not exist.",
        405: "405 Method Not Allowed – HTTP method not supported by this endpoint.",
        408: "408 Request Timeout – The server timed out waiting for the request.",
        429: "429 Too Many Requests – Rate limit exceeded. Try again later.",
        500: "500 Internal Server Error – The server encountered an unexpected condition.",
        502: "502 Bad Gateway – Invalid response from an upstream server.",
        503: "503 Service Unavailable – The server is temporarily unavailable.",
        504: "504 Gateway Timeout – The upstream server did not respond in time.",
    }
    message = messages.get(code, f"{code} Unexpected HTTP Error.")
    print(f"[HTTP ERROR] {message}")

def get_posts(limit: int = 5) -> None:
    print_section("STEP 1 – GET Request: Fetching Posts")
    url = f"{BASE_URL}/posts"

    print(f"  Sending GET request to: {url}\n")

    try:
        response = requests.get(url, timeout=10)

        if response.ok:
            posts = response.json()          # Parse JSON payload
            print(f"  Status : {response.status_code} OK")
            print(f"  Total posts retrieved: {len(posts)}")
            print(f"\n  Showing first {limit} post title(s):\n")

            for post in posts[:limit]:
                print(f"  [{post['id']:>3}] {post['title']}")

        else:
            handle_http_error(response)

    except requests.exceptions.ConnectionError:
        print("[CONNECTION ERROR] Could not reach the server. Check your internet connection.")
    except requests.exceptions.Timeout:
        print("[TIMEOUT ERROR] The request timed out. The server may be slow or unreachable.")
    except requests.exceptions.RequestException as exc:
        print(f"[REQUEST ERROR] An unexpected error occurred: {exc}")

def create_post(title: str, body: str, user_id: int) -> None:
    print_section("STEP 2 – POST Request: Creating a New Post")
    url = f"{BASE_URL}/posts"

    payload = {
        "title":  title,
        "body":   body,
        "userId": user_id,
    }

    print(f"  Sending POST request to : {url}")
    print(f"  Payload being sent:\n")
    print(json.dumps(payload, indent=6))
    print()

    try:
        response = requests.post(url, json=payload, timeout=10)

        if response.ok:
            created = response.json()
            print(f"  Status  : {response.status_code} Created ✓")
            print(f"\n  Server response (new resource):\n")
            print(json.dumps(created, indent=6))

        else:
            handle_http_error(response)

    except requests.exceptions.ConnectionError:
        print("[CONNECTION ERROR] Could not reach the server. Check your internet connection.")
    except requests.exceptions.Timeout:
        print("[TIMEOUT ERROR] The request timed out. The server may be slow or unreachable.")
    except requests.exceptions.RequestException as exc:
        print(f"[REQUEST ERROR] An unexpected error occurred: {exc}")

def demonstrate_error_handling() -> None:
    print_section("STEP 3 – Error Handling Demonstrations")

    # ── 3a: 404 Not Found ──────────────────────
    bad_endpoint = f"{BASE_URL}/posts/99999"
    print(f"  [Test 3a] GET request to a non-existent resource:")
    print(f"  URL: {bad_endpoint}\n")

    try:
        response = requests.get(bad_endpoint, timeout=10)
        if response.ok:
            print(f"  Unexpected success: {response.status_code}")
        else:
            handle_http_error(response)

    except requests.exceptions.RequestException as exc:
        print(f"[REQUEST ERROR] {exc}")

    # ── 3b: Invalid / unreachable URL ─────────────
    print(f"\n  [Test 3b] GET request to a completely invalid URL:")
    invalid_url = "https://this.url.does.not.exist.example/data"
    print(f"  URL: {invalid_url}\n")

    try:
        response = requests.get(invalid_url, timeout=5)
        if response.ok:
            print(f"  Unexpected success: {response.status_code}")
        else:
            handle_http_error(response)

    except requests.exceptions.ConnectionError:
        print("[CONNECTION ERROR] Could not reach the server – the URL is unreachable.")
    except requests.exceptions.Timeout:
        print("[TIMEOUT ERROR] The request timed out.")
    except requests.exceptions.RequestException as exc:
        print(f"[REQUEST ERROR] An unexpected error occurred: {exc}")


def main() -> None:
    print("\n" + "═" * 60)
    print("   Python Web Client – Module 08 Assignment")
    print("   Target API: https://jsonplaceholder.typicode.com")
    print("═" * 60)

    # Step 1 – GET
    get_posts(limit=5)

    # Step 2 – POST
    create_post(
        title="Learning Python Web Clients",
        body="Using the requests library to send HTTP requests is straightforward and powerful.",
        user_id=1,
    )

    # Step 3 – Error handling
    demonstrate_error_handling()

    print(f"\n{DIVIDER}")
    print("  All steps complete.")
    print(DIVIDER + "\n")


if __name__ == "__main__":
    main()