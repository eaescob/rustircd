#!/usr/bin/env python3
"""
Message throughput test for RustIRCd

Tests the server's message processing capacity by sending
messages at a specified rate and measuring latency.
"""

import argparse
import socket
import time
import threading
import statistics
from collections import deque

class IRCThroughputClient:
    def __init__(self, host, port, nick):
        self.host = host
        self.port = port
        self.nick = nick
        self.sock = None
        self.connected = False
        self.latencies = deque(maxlen=10000)
        
    def connect(self):
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(10)
            self.sock.connect((self.host, self.port))
            
            # Send NICK and USER
            self.sock.send(f"NICK {self.nick}\r\n".encode())
            self.sock.send(f"USER {self.nick} 0 * :Throughput Test\r\n".encode())
            
            # Wait for welcome
            data = self.sock.recv(4096)
            if b"001" in data:
                self.connected = True
                return True
            return False
        except Exception as e:
            print(f"Connection error: {e}")
            return False
    
    def send_message(self, target, message):
        if not self.connected:
            return None
        
        try:
            start = time.time()
            self.sock.send(f"PRIVMSG {target} :{message}\r\n".encode())
            
            # For this test, we'll assume message is sent successfully
            # In a real test, you'd want to verify with server response
            latency = time.time() - start
            self.latencies.append(latency)
            return latency
        except Exception as e:
            print(f"Send error: {e}")
            return None
    
    def disconnect(self):
        if self.sock:
            try:
                self.sock.send(b"QUIT\r\n")
                self.sock.close()
            except:
                pass

class ThroughputTest:
    def __init__(self, host, port, message_rate, duration):
        self.host = host
        self.port = port
        self.message_rate = message_rate
        self.duration = duration
        self.clients = []
        self.messages_sent = 0
        self.messages_failed = 0
        self.lock = threading.Lock()
        
    def sender_thread(self, client_id, messages_per_client):
        nick = f"sender{client_id}"
        client = IRCThroughputClient(self.host, self.port, nick)
        
        if not client.connect():
            print(f"Client {nick} failed to connect")
            return
        
        with self.lock:
            self.clients.append(client)
        
        # Send messages
        delay = 1.0 / (self.message_rate / 10)  # Assume 10 clients
        
        for i in range(messages_per_client):
            latency = client.send_message("#test", f"Message {i} from {nick}")
            
            with self.lock:
                if latency is not None:
                    self.messages_sent += 1
                else:
                    self.messages_failed += 1
            
            if delay > 0:
                time.sleep(delay)
    
    def run(self):
        print(f"Starting throughput test")
        print(f"Target: {self.host}:{self.port}")
        print(f"Message rate: {self.message_rate}/sec")
        print(f"Duration: {self.duration}s")
        print("-" * 60)
        
        num_clients = 10
        messages_per_client = (self.message_rate * self.duration) // num_clients
        
        start_time = time.time()
        threads = []
        
        # Start sender threads
        for i in range(num_clients):
            thread = threading.Thread(
                target=self.sender_thread,
                args=(i, messages_per_client)
            )
            thread.start()
            threads.append(thread)
        
        # Monitor progress
        last_count = 0
        while any(t.is_alive() for t in threads):
            time.sleep(1)
            with self.lock:
                current = self.messages_sent
            rate = current - last_count
            last_count = current
            print(f"Sent: {current}, Rate: {rate}/sec, Failed: {self.messages_failed}")
        
        # Wait for completion
        for thread in threads:
            thread.join()
        
        total_time = time.time() - start_time
        
        # Collect latency statistics
        all_latencies = []
        for client in self.clients:
            all_latencies.extend(client.latencies)
        
        # Print results
        print("\n" + "=" * 60)
        print("RESULTS")
        print("=" * 60)
        print(f"Messages sent:      {self.messages_sent}")
        print(f"Messages failed:    {self.messages_failed}")
        print(f"Total time:         {total_time:.2f}s")
        print(f"Actual rate:        {self.messages_sent / total_time:.1f}/sec")
        
        if all_latencies:
            print(f"\nLatency statistics:")
            print(f"  Min:     {min(all_latencies) * 1000:.2f}ms")
            print(f"  Max:     {max(all_latencies) * 1000:.2f}ms")
            print(f"  Mean:    {statistics.mean(all_latencies) * 1000:.2f}ms")
            print(f"  Median:  {statistics.median(all_latencies) * 1000:.2f}ms")
            
            if len(all_latencies) > 1:
                print(f"  Stdev:   {statistics.stdev(all_latencies) * 1000:.2f}ms")
            
            # Percentiles
            sorted_lat = sorted(all_latencies)
            p50 = sorted_lat[len(sorted_lat) * 50 // 100]
            p95 = sorted_lat[len(sorted_lat) * 95 // 100]
            p99 = sorted_lat[len(sorted_lat) * 99 // 100]
            print(f"  P50:     {p50 * 1000:.2f}ms")
            print(f"  P95:     {p95 * 1000:.2f}ms")
            print(f"  P99:     {p99 * 1000:.2f}ms")
        
        # Cleanup
        for client in self.clients:
            client.disconnect()
        
        print("\nTest complete!")

def main():
    parser = argparse.ArgumentParser(description='IRC message throughput test')
    parser.add_argument('--host', default='localhost', help='IRC server host')
    parser.add_argument('--port', type=int, default=6667, help='IRC server port')
    parser.add_argument('--rate', type=int, default=1000, help='Messages per second')
    parser.add_argument('--duration', type=int, default=60, help='Test duration in seconds')
    
    args = parser.parse_args()
    
    test = ThroughputTest(args.host, args.port, args.rate, args.duration)
    test.run()

if __name__ == '__main__':
    main()


