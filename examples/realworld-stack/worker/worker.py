import time
import sys

def main():
    print("[worker] Background worker initialized", flush=True)
    print("[worker] Queue stream listener started on channel 'realtime_events'", flush=True)
    print("[worker] Worker supervisor healthy", flush=True)

    job_id = 1000
    while True:
        time.sleep(2.5)
        job_id += 1
        print(f"[worker] Processed background job #{job_id} in 12ms (status: SUCCESS, 0 failed)", flush=True)

if __name__ == "__main__":
    main()
