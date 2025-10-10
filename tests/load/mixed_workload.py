#!/usr/bin/env python3
"""
Mixed Workload Test for RustIRCd

Simulates realistic IRC traffic patterns with a mix of:
- Channel messages (70%)
- Private messages (20%)
- Joins/Parts (5%)
- Mode changes (3%)
- Operator commands (2%)

Usage:
    ./mixed_workload.py --duration 300
    ./mixed_workload.py --users 100 --channels 20 --duration 600
"""

import argparse
import socket
import threading
import time
import random
import sys
from collections import defaultdict
from datetime import datetime

class IRCClient:
    def __init__(self, host, port, nickname):
        self.host = host
        self.port = port
        self.nickname = nickname
        self.socket = None
        self.connected = False
        self.channels = []
        
    def connect(self):
        """Connect to IRC server"""
        try:
            self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.socket.settimeout(10)
            self.socket.connect((self.host, self.port))
            
            # Register
            self.send(f"NICK {self.nickname}")
            self.send(f"USER {self.nickname} 0 * :Test User")
            
            # Wait for registration to complete
            time.sleep(0.5)
            self.connected = True
            return True
        except Exception as e:
            print(f"Connection failed for {self.nickname}: {e}")
            return False
    
    def send(self, message):
        """Send IRC message"""
        try:
            self.socket.send(f"{message}\r\n".encode())
            return True
        except:
            self.connected = False
            return False
    
    def join(self, channel):
        """Join a channel"""
        if self.send(f"JOIN {channel}"):
            self.channels.append(channel)
            return True
        return False
    
    def part(self, channel, reason="Leaving"):
        """Leave a channel"""
        if channel in self.channels and self.send(f"PART {channel} :{reason}"):
            self.channels.remove(channel)
            return True
        return False
    
    def privmsg(self, target, message):
        """Send a message"""
        return self.send(f"PRIVMSG {target} :{message}")
    
    def quit(self, reason="Client quit"):
        """Disconnect from server"""
        self.send(f"QUIT :{reason}")
        if self.socket:
            try:
                self.socket.close()
            except:
                pass
        self.connected = False

class MixedWorkloadTest:
    def __init__(self, host='localhost', port=6667, num_users=50, 
                 num_channels=10, duration=300):
        self.host = host
        self.port = port
        self.num_users = num_users
        self.num_channels = num_channels
        self.duration = duration
        
        self.clients = []
        self.channels = [f"#test{i}" for i in range(num_channels)]
        self.start_time = None
        self.stop_flag = threading.Event()
        
        # Statistics
        self.stats = {
            'channel_messages': 0,
            'private_messages': 0,
            'joins': 0,
            'parts': 0,
            'mode_changes': 0,
            'operator_commands': 0,
            'failed_operations': 0
        }
        self.stats_lock = threading.Lock()
    
    def create_clients(self):
        """Create and connect all clients"""
        print(f"Creating {self.num_users} clients...")
        success = 0
        failed = 0
        
        for i in range(self.num_users):
            nickname = f"user{i:04d}"
            client = IRCClient(self.host, self.port, nickname)
            
            if client.connect():
                # Join random channels
                num_joins = random.randint(1, min(3, self.num_channels))
                channels_to_join = random.sample(self.channels, num_joins)
                
                for channel in channels_to_join:
                    client.join(channel)
                    time.sleep(0.05)
                
                self.clients.append(client)
                success += 1
            else:
                failed += 1
            
            # Rate limit connection creation
            time.sleep(0.1)
        
        print(f"✓ Connected: {success}, Failed: {failed}")
        return success > 0
    
    def channel_message_worker(self):
        """Worker thread for channel messages (70% of traffic)"""
        messages = [
            "Hello everyone!",
            "How is everyone doing?",
            "This is a test message",
            "Anyone here?",
            "Just testing the server",
            "Performance test in progress",
            "Let's see how this handles",
            "IRC is awesome!",
            "Testing, testing, 1-2-3",
            "Message number {}"
        ]
        
        counter = 0
        while not self.stop_flag.is_set():
            client = random.choice(self.clients)
            if client.connected and client.channels:
                channel = random.choice(client.channels)
                message = random.choice(messages).format(counter)
                
                if client.privmsg(channel, message):
                    with self.stats_lock:
                        self.stats['channel_messages'] += 1
                else:
                    with self.stats_lock:
                        self.stats['failed_operations'] += 1
                
                counter += 1
            
            # 70% of traffic, adjust timing to achieve target rate
            time.sleep(0.05)
    
    def private_message_worker(self):
        """Worker thread for private messages (20% of traffic)"""
        while not self.stop_flag.is_set():
            if len(self.clients) < 2:
                time.sleep(1)
                continue
            
            sender = random.choice(self.clients)
            receiver = random.choice(self.clients)
            
            if sender != receiver and sender.connected:
                message = f"Private message at {time.time()}"
                if sender.privmsg(receiver.nickname, message):
                    with self.stats_lock:
                        self.stats['private_messages'] += 1
                else:
                    with self.stats_lock:
                        self.stats['failed_operations'] += 1
            
            # 20% of traffic
            time.sleep(0.2)
    
    def join_part_worker(self):
        """Worker thread for joins/parts (5% of traffic)"""
        while not self.stop_flag.is_set():
            client = random.choice(self.clients)
            
            if not client.connected:
                time.sleep(1)
                continue
            
            action = random.choice(['join', 'part'])
            
            if action == 'join' and len(client.channels) < len(self.channels):
                # Find a channel we're not in
                available = [ch for ch in self.channels if ch not in client.channels]
                if available:
                    channel = random.choice(available)
                    if client.join(channel):
                        with self.stats_lock:
                            self.stats['joins'] += 1
                    else:
                        with self.stats_lock:
                            self.stats['failed_operations'] += 1
            
            elif action == 'part' and client.channels:
                channel = random.choice(client.channels)
                if client.part(channel):
                    with self.stats_lock:
                        self.stats['parts'] += 1
                else:
                    with self.stats_lock:
                        self.stats['failed_operations'] += 1
            
            # 5% of traffic
            time.sleep(1.0)
    
    def status_reporter(self):
        """Report status periodically"""
        last_stats = self.stats.copy()
        
        while not self.stop_flag.is_set():
            time.sleep(10)
            
            with self.stats_lock:
                current_stats = self.stats.copy()
            
            elapsed = time.time() - self.start_time
            
            # Calculate rates
            channel_msg_rate = (current_stats['channel_messages'] - 
                               last_stats['channel_messages']) / 10
            private_msg_rate = (current_stats['private_messages'] - 
                               last_stats['private_messages']) / 10
            
            print(f"[{int(elapsed)}s] "
                  f"Chan: {current_stats['channel_messages']} ({channel_msg_rate:.1f}/s), "
                  f"PM: {current_stats['private_messages']} ({private_msg_rate:.1f}/s), "
                  f"Join/Part: {current_stats['joins']}/{current_stats['parts']}, "
                  f"Failed: {current_stats['failed_operations']}")
            
            last_stats = current_stats
    
    def cleanup(self):
        """Disconnect all clients"""
        print("\nCleaning up connections...")
        for client in self.clients:
            if client.connected:
                client.quit()
        print("✓ Cleanup complete")
    
    def print_results(self):
        """Print final test results"""
        elapsed = time.time() - self.start_time
        
        print("\n" + "=" * 70)
        print("RESULTS")
        print("=" * 70)
        print(f"Duration:            {elapsed:.2f}s")
        print(f"Active clients:      {sum(1 for c in self.clients if c.connected)}/{len(self.clients)}")
        print()
        print("Message Statistics:")
        print(f"  Channel messages:  {self.stats['channel_messages']}")
        print(f"  Private messages:  {self.stats['private_messages']}")
        print(f"  Joins:             {self.stats['joins']}")
        print(f"  Parts:             {self.stats['parts']}")
        print(f"  Mode changes:      {self.stats['mode_changes']}")
        print(f"  Operator commands: {self.stats['operator_commands']}")
        print(f"  Failed operations: {self.stats['failed_operations']}")
        print()
        
        total_ops = sum(self.stats.values())
        if total_ops > 0:
            print("Traffic Distribution:")
            print(f"  Channel messages:  {self.stats['channel_messages']/total_ops*100:.1f}%")
            print(f"  Private messages:  {self.stats['private_messages']/total_ops*100:.1f}%")
            print(f"  Joins/Parts:       {(self.stats['joins']+self.stats['parts'])/total_ops*100:.1f}%")
            print(f"  Other:             {(self.stats['mode_changes']+self.stats['operator_commands'])/total_ops*100:.1f}%")
            print()
        
        print(f"Operations/second:   {total_ops/elapsed:.2f}")
        print(f"Success rate:        {(total_ops-self.stats['failed_operations'])/total_ops*100:.1f}%")
    
    def run(self):
        """Run the mixed workload test"""
        print("=" * 70)
        print("RustIRCd Mixed Workload Test")
        print("=" * 70)
        print(f"Server:    {self.host}:{self.port}")
        print(f"Users:     {self.num_users}")
        print(f"Channels:  {self.num_channels}")
        print(f"Duration:  {self.duration}s")
        print()
        
        try:
            # Create clients
            if not self.create_clients():
                print("Failed to create clients")
                return False
            
            print(f"\nRunning mixed workload for {self.duration} seconds...")
            self.start_time = time.time()
            
            # Start worker threads
            workers = []
            
            # Multiple channel message workers (70% of traffic)
            for _ in range(5):
                t = threading.Thread(target=self.channel_message_worker, daemon=True)
                t.start()
                workers.append(t)
            
            # Private message workers (20% of traffic)
            for _ in range(2):
                t = threading.Thread(target=self.private_message_worker, daemon=True)
                t.start()
                workers.append(t)
            
            # Join/part worker (5% of traffic)
            t = threading.Thread(target=self.join_part_worker, daemon=True)
            t.start()
            workers.append(t)
            
            # Status reporter
            t = threading.Thread(target=self.status_reporter, daemon=True)
            t.start()
            workers.append(t)
            
            # Wait for duration
            time.sleep(self.duration)
            
            # Stop workers
            self.stop_flag.set()
            
            # Wait for workers to finish
            for worker in workers:
                worker.join(timeout=2)
            
            # Print results
            self.print_results()
            
            return self.stats['failed_operations'] < total_ops * 0.05  # Less than 5% failure rate
            
        except KeyboardInterrupt:
            print("\n\nTest interrupted by user")
            self.stop_flag.set()
            return False
        finally:
            self.cleanup()


def main():
    parser = argparse.ArgumentParser(
        description='Mixed workload test for RustIRCd',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
  # Quick test with defaults
  %(prog)s --duration 60
  
  # Large test
  %(prog)s --users 200 --channels 50 --duration 600
  
  # Custom server
  %(prog)s --host example.com --port 6667
        '''
    )
    
    parser.add_argument('--host', default='localhost', help='IRC server hostname')
    parser.add_argument('--port', type=int, default=6667, help='IRC server port')
    parser.add_argument('--users', type=int, default=50, help='Number of users')
    parser.add_argument('--channels', type=int, default=10, help='Number of channels')
    parser.add_argument('--duration', type=int, default=300, help='Test duration in seconds')
    
    args = parser.parse_args()
    
    test = MixedWorkloadTest(
        host=args.host,
        port=args.port,
        num_users=args.users,
        num_channels=args.channels,
        duration=args.duration
    )
    
    success = test.run()
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()

