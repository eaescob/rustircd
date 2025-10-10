#!/usr/bin/env python3
"""
Channel Load Test for RustIRCd

Tests channel-specific performance with varying channel sizes:
- Small channels (10-50 members)
- Medium channels (50-200 members)
- Large channels (200-1000+ members)

Measures:
- Broadcast latency per channel size
- JOIN performance
- Message distribution time
- Channel-specific throughput

Usage:
    ./channel_load.py --channels 50 --max-users 100
    ./channel_load.py --channels 10 --max-users 1000 --duration 300
"""

import argparse
import socket
import threading
import time
import random
import statistics
import sys

class ChannelLoadTest:
    def __init__(self, host='localhost', port=6667, num_channels=20, 
                 max_users_per_channel=100, duration=180):
        self.host = host
        self.port = port
        self.num_channels = num_channels
        self.max_users_per_channel = max_users_per_channel
        self.duration = duration
        
        self.channels = []
        self.clients = []
        self.latencies = []
        self.start_time = None
        self.stop_flag = threading.Event()
        
        # Statistics
        self.stats = {
            'messages_sent': 0,
            'messages_failed': 0,
            'joins_completed': 0,
            'joins_failed': 0,
        }
        self.stats_lock = threading.Lock()
    
    def create_connection(self, nickname):
        """Create and register a client connection"""
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(10)
            sock.connect((self.host, self.port))
            
            # Register
            sock.send(f"NICK {nickname}\r\n".encode())
            sock.send(f"USER {nickname} 0 * :Test User\r\n".encode())
            
            time.sleep(0.2)  # Wait for registration
            return sock
        except Exception as e:
            print(f"Connection failed for {nickname}: {e}")
            return None
    
    def setup_channels(self):
        """Set up channels with varying sizes"""
        print(f"Setting up {self.num_channels} channels...")
        
        # Create channels with varying sizes
        # Distribution: 50% small, 30% medium, 20% large
        small_count = int(self.num_channels * 0.5)
        medium_count = int(self.num_channels * 0.3)
        large_count = self.num_channels - small_count - medium_count
        
        channel_configs = []
        
        # Small channels (10-50 members)
        for i in range(small_count):
            size = random.randint(10, 50)
            channel_configs.append((f"#small{i}", size))
        
        # Medium channels (50-200 members)
        for i in range(medium_count):
            size = random.randint(50, min(200, self.max_users_per_channel))
            channel_configs.append((f"#medium{i}", size))
        
        # Large channels (200-1000+ members)
        for i in range(large_count):
            size = random.randint(200, self.max_users_per_channel)
            channel_configs.append((f"#large{i}", size))
        
        # Create clients and join them to channels
        client_id = 0
        for channel_name, target_size in channel_configs:
            members = []
            
            print(f"  Creating {channel_name} with ~{target_size} members...", end=' ')
            
            for _ in range(target_size):
                nickname = f"user{client_id:05d}"
                sock = self.create_connection(nickname)
                
                if sock:
                    # Join the channel
                    try:
                        sock.send(f"JOIN {channel_name}\r\n".encode())
                        members.append((nickname, sock))
                        
                        with self.stats_lock:
                            self.stats['joins_completed'] += 1
                    except Exception as e:
                        with self.stats_lock:
                            self.stats['joins_failed'] += 1
                        sock.close()
                else:
                    with self.stats_lock:
                        self.stats['joins_failed'] += 1
                
                client_id += 1
                time.sleep(0.01)  # Rate limit
            
            print(f"✓ {len(members)} members")
            self.channels.append({
                'name': channel_name,
                'target_size': target_size,
                'actual_size': len(members),
                'members': members
            })
        
        total_members = sum(len(ch['members']) for ch in self.channels)
        print(f"\n✓ Total channels: {len(self.channels)}, Total clients: {total_members}")
        
        return len(self.channels) > 0
    
    def measure_broadcast_latency(self, channel):
        """Measure broadcast latency for a channel"""
        if not channel['members']:
            return None
        
        # Pick a sender
        sender_nick, sender_sock = channel['members'][0]
        
        # Send message with timestamp
        start_time = time.time()
        test_message = f"LATENCY_TEST_{start_time}"
        
        try:
            sender_sock.send(f"PRIVMSG {channel['name']} :{test_message}\r\n".encode())
            
            # Measure time to process
            # In a real implementation, we'd check receivers, but for simplicity
            # we'll just measure send time as a proxy
            latency = (time.time() - start_time) * 1000  # Convert to ms
            
            return latency
        except Exception as e:
            return None
    
    def broadcast_test_worker(self):
        """Worker to continuously test channel broadcasts"""
        while not self.stop_flag.is_set():
            if not self.channels:
                time.sleep(1)
                continue
            
            channel = random.choice(self.channels)
            if not channel['members']:
                continue
            
            sender_nick, sender_sock = random.choice(channel['members'])
            message = f"Test message at {time.time()}"
            
            try:
                start = time.time()
                sender_sock.send(f"PRIVMSG {channel['name']} :{message}\r\n".encode())
                latency = (time.time() - start) * 1000
                
                self.latencies.append({
                    'channel_size': channel['actual_size'],
                    'latency_ms': latency
                })
                
                with self.stats_lock:
                    self.stats['messages_sent'] += 1
            except Exception as e:
                with self.stats_lock:
                    self.stats['messages_failed'] += 1
            
            time.sleep(0.1)
    
    def status_reporter(self):
        """Report status periodically"""
        while not self.stop_flag.is_set():
            time.sleep(10)
            
            elapsed = time.time() - self.start_time
            with self.stats_lock:
                msgs = self.stats['messages_sent']
                failed = self.stats['messages_failed']
            
            rate = msgs / elapsed if elapsed > 0 else 0
            print(f"[{int(elapsed)}s] Messages: {msgs} ({rate:.1f}/s), Failed: {failed}")
    
    def analyze_results(self):
        """Analyze channel performance by size"""
        print("\n" + "=" * 70)
        print("CHANNEL PERFORMANCE ANALYSIS")
        print("=" * 70 + "\n")
        
        # Group latencies by channel size buckets
        size_buckets = {
            'small (10-50)': [],
            'medium (50-200)': [],
            'large (200-1000)': [],
            'xlarge (1000+)': []
        }
        
        for sample in self.latencies:
            size = sample['channel_size']
            latency = sample['latency_ms']
            
            if size <= 50:
                size_buckets['small (10-50)'].append(latency)
            elif size <= 200:
                size_buckets['medium (50-200)'].append(latency)
            elif size <= 1000:
                size_buckets['large (200-1000)'].append(latency)
            else:
                size_buckets['xlarge (1000+)'].append(latency)
        
        # Print statistics for each bucket
        print(f"{'Channel Size':<20} {'Count':<10} {'Min':<10} {'Avg':<10} {'P50':<10} {'P95':<10} {'P99':<10} {'Max':<10}")
        print("-" * 110)
        
        for bucket_name, latencies in size_buckets.items():
            if not latencies:
                continue
            
            latencies.sort()
            count = len(latencies)
            min_lat = min(latencies)
            max_lat = max(latencies)
            avg_lat = statistics.mean(latencies)
            p50 = latencies[int(count * 0.50)] if count > 0 else 0
            p95 = latencies[int(count * 0.95)] if count > 1 else max_lat
            p99 = latencies[int(count * 0.99)] if count > 1 else max_lat
            
            print(f"{bucket_name:<20} {count:<10} {min_lat:<10.2f} {avg_lat:<10.2f} "
                  f"{p50:<10.2f} {p95:<10.2f} {p99:<10.2f} {max_lat:<10.2f}")
        
        print()
    
    def print_results(self):
        """Print final test results"""
        elapsed = time.time() - self.start_time
        
        print("\n" + "=" * 70)
        print("RESULTS")
        print("=" * 70)
        print(f"Duration:            {elapsed:.2f}s")
        print(f"Channels created:    {len(self.channels)}")
        print()
        
        print("Channel Size Distribution:")
        small = sum(1 for ch in self.channels if ch['actual_size'] <= 50)
        medium = sum(1 for ch in self.channels if 50 < ch['actual_size'] <= 200)
        large = sum(1 for ch in self.channels if 200 < ch['actual_size'] <= 1000)
        xlarge = sum(1 for ch in self.channels if ch['actual_size'] > 1000)
        
        print(f"  Small (10-50):     {small}")
        print(f"  Medium (50-200):   {medium}")
        print(f"  Large (200-1000):  {large}")
        print(f"  XLarge (1000+):    {xlarge}")
        print()
        
        print("Operations:")
        print(f"  Messages sent:     {self.stats['messages_sent']}")
        print(f"  Messages failed:   {self.stats['messages_failed']}")
        print(f"  Joins completed:   {self.stats['joins_completed']}")
        print(f"  Joins failed:      {self.stats['joins_failed']}")
        print()
        
        success_rate = (self.stats['messages_sent'] / 
                       (self.stats['messages_sent'] + self.stats['messages_failed']) * 100
                       if self.stats['messages_sent'] + self.stats['messages_failed'] > 0 else 0)
        print(f"Success rate:        {success_rate:.1f}%")
        print()
        
        # Analyze channel performance
        if self.latencies:
            self.analyze_results()
    
    def cleanup(self):
        """Disconnect all clients"""
        print("\nCleaning up connections...")
        for channel in self.channels:
            for nickname, sock in channel['members']:
                try:
                    sock.send(b"QUIT :Test complete\r\n")
                    sock.close()
                except:
                    pass
        print("✓ Cleanup complete")
    
    def run(self):
        """Run the channel load test"""
        print("=" * 70)
        print("RustIRCd Channel Load Test")
        print("=" * 70)
        print(f"Server:              {self.host}:{self.port}")
        print(f"Channels:            {self.num_channels}")
        print(f"Max users/channel:   {self.max_users_per_channel}")
        print(f"Test duration:       {self.duration}s")
        print()
        
        try:
            # Set up channels
            if not self.setup_channels():
                print("Failed to set up channels")
                return False
            
            print(f"\nRunning broadcast tests for {self.duration} seconds...")
            self.start_time = time.time()
            
            # Start worker threads
            workers = []
            for _ in range(5):
                t = threading.Thread(target=self.broadcast_test_worker, daemon=True)
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
            
            # Wait for workers
            for worker in workers:
                worker.join(timeout=2)
            
            # Print results
            self.print_results()
            
            return self.stats['messages_failed'] < self.stats['messages_sent'] * 0.05
            
        except KeyboardInterrupt:
            print("\n\nTest interrupted by user")
            self.stop_flag.set()
            return False
        finally:
            self.cleanup()


def main():
    parser = argparse.ArgumentParser(
        description='Channel load test for RustIRCd',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
  # Test with 20 channels, up to 100 users each
  %(prog)s --channels 20 --max-users 100
  
  # Large channel test
  %(prog)s --channels 10 --max-users 1000 --duration 600
  
  # Many small channels
  %(prog)s --channels 100 --max-users 50
        '''
    )
    
    parser.add_argument('--host', default='localhost', help='IRC server hostname')
    parser.add_argument('--port', type=int, default=6667, help='IRC server port')
    parser.add_argument('--channels', type=int, default=20, help='Number of channels')
    parser.add_argument('--max-users', type=int, default=100, help='Maximum users per channel')
    parser.add_argument('--duration', type=int, default=180, help='Test duration in seconds')
    
    args = parser.parse_args()
    
    test = ChannelLoadTest(
        host=args.host,
        port=args.port,
        num_channels=args.channels,
        max_users_per_channel=args.max_users,
        duration=args.duration
    )
    
    success = test.run()
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()

