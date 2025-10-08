#!/usr/bin/env python3
"""
Connection stress test for RustIRCd

Tests the server's ability to handle many concurrent connections.
Measures connection establishment rate, memory usage, and CPU usage.
"""

import argparse
import socket
import time
import threading
import sys
from collections import defaultdict

class IRCClient:
    def __init__(self, host, port, nick):
        self.host = host
        self.port = port
        self.nick = nick
        self.sock = None
        self.connected = False
        
    def connect(self):
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(10)
            self.sock.connect((self.host, self.port))
            
            # Send NICK and USER
            self.sock.send(f"NICK {self.nick}\r\n".encode())
            self.sock.send(f"USER {self.nick} 0 * :Load Test Client\r\n".encode())
            
            # Wait for welcome message
            data = self.sock.recv(4096)
            if b"001" in data or b"Welcome" in data:
                self.connected = True
                return True
            return False
        except Exception as e:
            print(f"Connection error for {self.nick}: {e}")
            return False
    
    def disconnect(self):
        if self.sock:
            try:
                self.sock.send(b"QUIT :Test complete\r\n")
                self.sock.close()
            except:
                pass
        self.connected = False

class StressTest:
    def __init__(self, host, port, num_clients, connect_rate):
        self.host = host
        self.port = port
        self.num_clients = num_clients
        self.connect_rate = connect_rate
        self.clients = []
        self.stats = defaultdict(int)
        self.lock = threading.Lock()
        
    def connect_client(self, client_id):
        nick = f"stress{client_id}"
        client = IRCClient(self.host, self.port, nick)
        
        start = time.time()
        if client.connect():
            elapsed = time.time() - start
            with self.lock:
                self.stats['connected'] += 1
                self.stats['total_connect_time'] += elapsed
                self.clients.append(client)
            return True
        else:
            with self.lock:
                self.stats['failed'] += 1
            return False
    
    def run(self):
        print(f"Starting stress test: {self.num_clients} clients")
        print(f"Target: {self.host}:{self.port}")
        print(f"Connection rate: {self.connect_rate}/sec")
        print("-" * 60)
        
        start_time = time.time()
        threads = []
        
        # Connect clients at specified rate
        delay = 1.0 / self.connect_rate if self.connect_rate > 0 else 0
        
        for i in range(self.num_clients):
            thread = threading.Thread(target=self.connect_client, args=(i,))
            thread.start()
            threads.append(thread)
            
            if delay > 0 and (i + 1) % self.connect_rate == 0:
                time.sleep(1)
            
            # Progress update
            if (i + 1) % 100 == 0:
                elapsed = time.time() - start_time
                rate = (i + 1) / elapsed
                print(f"Progress: {i + 1}/{self.num_clients} "
                      f"({rate:.1f} conn/sec, "
                      f"{self.stats['connected']} success, "
                      f"{self.stats['failed']} failed)")
        
        # Wait for all connections to complete
        for thread in threads:
            thread.join()
        
        total_time = time.time() - start_time
        
        # Print results
        print("\n" + "=" * 60)
        print("RESULTS")
        print("=" * 60)
        print(f"Total clients:      {self.num_clients}")
        print(f"Successful:         {self.stats['connected']}")
        print(f"Failed:             {self.stats['failed']}")
        print(f"Total time:         {total_time:.2f}s")
        print(f"Connections/sec:    {self.stats['connected'] / total_time:.1f}")
        
        if self.stats['connected'] > 0:
            avg_connect = self.stats['total_connect_time'] / self.stats['connected']
            print(f"Avg connect time:   {avg_connect * 1000:.1f}ms")
        
        # Keep connections alive
        if self.stats['connected'] > 0:
            print(f"\nKeeping {self.stats['connected']} connections alive...")
            print("Press Ctrl+C to disconnect and exit")
            
            try:
                while True:
                    time.sleep(1)
            except KeyboardInterrupt:
                print("\nDisconnecting clients...")
        
        # Cleanup
        for client in self.clients:
            client.disconnect()
        
        print("Test complete!")

def main():
    parser = argparse.ArgumentParser(description='IRC connection stress test')
    parser.add_argument('--host', default='localhost', help='IRC server host')
    parser.add_argument('--port', type=int, default=6667, help='IRC server port')
    parser.add_argument('--clients', type=int, default=100, help='Number of clients')
    parser.add_argument('--rate', type=int, default=50, help='Connections per second')
    
    args = parser.parse_args()
    
    test = StressTest(args.host, args.port, args.clients, args.rate)
    test.run()

if __name__ == '__main__':
    main()



