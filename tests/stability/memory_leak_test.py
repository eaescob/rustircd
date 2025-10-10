#!/usr/bin/env python3
"""
Memory Leak Detection Test for RustIRCd

Runs the server for an extended period while monitoring memory usage
to detect memory leaks. Simulates realistic workload and plots memory
usage over time.

Usage:
    ./memory_leak_test.py --duration 3600  # 1 hour test
    ./memory_leak_test.py --duration 86400  # 24 hour test
"""

import argparse
import socket
import time
import subprocess
import psutil
import sys
import json
from datetime import datetime
from pathlib import Path

class MemoryLeakTest:
    def __init__(self, host='localhost', port=6667, duration=3600, sample_interval=60):
        self.host = host
        self.port = port
        self.duration = duration
        self.sample_interval = sample_interval
        self.process = None
        self.memory_samples = []
        self.start_time = None
        
    def start_server(self):
        """Start the RustIRCd server"""
        print("Starting RustIRCd server...")
        self.process = subprocess.Popen(
            ['cargo', 'run', '--release'],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        # Wait for server to be ready
        print("Waiting for server to start...")
        for _ in range(30):
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(1)
                sock.connect((self.host, self.port))
                sock.close()
                print("✓ Server is ready")
                return True
            except (ConnectionRefusedError, socket.timeout):
                time.sleep(1)
        
        print("✗ Server failed to start")
        return False
    
    def stop_server(self):
        """Stop the RustIRCd server"""
        if self.process:
            print("\nStopping server...")
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
    
    def get_memory_usage(self):
        """Get current memory usage of the server process"""
        if not self.process:
            return None
        
        try:
            process = psutil.Process(self.process.pid)
            mem_info = process.memory_info()
            return {
                'rss': mem_info.rss / 1024 / 1024,  # MB
                'vms': mem_info.vms / 1024 / 1024,  # MB
                'timestamp': time.time() - self.start_time
            }
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            return None
    
    def create_test_connection(self):
        """Create a test IRC connection"""
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5)
            sock.connect((self.host, self.port))
            return sock
        except Exception as e:
            print(f"Connection failed: {e}")
            return None
    
    def simulate_workload(self, sock, user_id):
        """Simulate realistic IRC workload"""
        try:
            # Register
            sock.send(f"NICK testuser{user_id}\r\n".encode())
            sock.send(f"USER test{user_id} 0 * :Test User {user_id}\r\n".encode())
            
            # Join channel
            sock.send(b"JOIN #test\r\n")
            
            # Send some messages
            for i in range(5):
                sock.send(f"PRIVMSG #test :Test message {i}\r\n".encode())
                time.sleep(0.1)
            
            # Clean disconnect
            sock.send(b"QUIT :Test complete\r\n")
            sock.close()
        except Exception as e:
            print(f"Workload simulation error: {e}")
    
    def run_continuous_workload(self):
        """Run continuous workload throughout the test"""
        connections_per_cycle = 10
        cycle_interval = 60  # seconds
        
        cycle = 0
        while time.time() - self.start_time < self.duration:
            # Create and use connections
            for i in range(connections_per_cycle):
                sock = self.create_test_connection()
                if sock:
                    self.simulate_workload(sock, cycle * connections_per_cycle + i)
            
            cycle += 1
            time.sleep(cycle_interval)
    
    def monitor_memory(self):
        """Monitor memory usage throughout the test"""
        print(f"\nMonitoring memory usage for {self.duration} seconds...")
        print(f"Sampling every {self.sample_interval} seconds\n")
        print(f"{'Time':<10} {'RSS (MB)':<15} {'VMS (MB)':<15} {'Status'}")
        print("-" * 70)
        
        self.start_time = time.time()
        next_sample = self.start_time
        
        while time.time() - self.start_time < self.duration:
            current_time = time.time()
            
            # Sample memory at intervals
            if current_time >= next_sample:
                mem = self.get_memory_usage()
                if mem:
                    self.memory_samples.append(mem)
                    
                    elapsed = int(mem['timestamp'])
                    status = "OK"
                    
                    # Check for memory growth
                    if len(self.memory_samples) > 10:
                        recent = [s['rss'] for s in self.memory_samples[-10:]]
                        first = self.memory_samples[0]['rss']
                        
                        # Check if memory is growing linearly
                        growth = (mem['rss'] - first) / first * 100
                        if growth > 50:  # More than 50% growth
                            status = "WARNING: Memory growth detected"
                    
                    print(f"{elapsed:<10} {mem['rss']:<15.2f} {mem['vms']:<15.2f} {status}")
                
                next_sample = current_time + self.sample_interval
            
            time.sleep(1)
    
    def analyze_results(self):
        """Analyze memory samples for leaks"""
        if len(self.memory_samples) < 10:
            print("\n✗ Not enough samples for analysis")
            return False
        
        print("\n" + "=" * 70)
        print("MEMORY LEAK ANALYSIS")
        print("=" * 70 + "\n")
        
        rss_values = [s['rss'] for s in self.memory_samples]
        timestamps = [s['timestamp'] for s in self.memory_samples]
        
        initial_rss = rss_values[0]
        final_rss = rss_values[-1]
        max_rss = max(rss_values)
        min_rss = min(rss_values)
        avg_rss = sum(rss_values) / len(rss_values)
        
        print(f"Initial RSS:     {initial_rss:.2f} MB")
        print(f"Final RSS:       {final_rss:.2f} MB")
        print(f"Maximum RSS:     {max_rss:.2f} MB")
        print(f"Minimum RSS:     {min_rss:.2f} MB")
        print(f"Average RSS:     {avg_rss:.2f} MB")
        print()
        
        # Calculate growth rate
        total_growth = final_rss - initial_rss
        growth_percentage = (total_growth / initial_rss) * 100
        time_hours = timestamps[-1] / 3600
        growth_per_hour = total_growth / time_hours if time_hours > 0 else 0
        
        print(f"Total growth:    {total_growth:+.2f} MB ({growth_percentage:+.1f}%)")
        print(f"Growth rate:     {growth_per_hour:+.2f} MB/hour")
        print()
        
        # Determine if there's a leak
        leak_detected = False
        if growth_percentage > 100:  # More than 100% growth
            print("🔴 SEVERE MEMORY LEAK DETECTED")
            print(f"   Memory more than doubled ({growth_percentage:.1f}% growth)")
            leak_detected = True
        elif growth_percentage > 50:  # More than 50% growth
            print("🟡 POSSIBLE MEMORY LEAK")
            print(f"   Significant memory growth detected ({growth_percentage:.1f}%)")
            leak_detected = True
        elif growth_percentage > 20:  # More than 20% growth
            print("🟡 MODERATE MEMORY GROWTH")
            print(f"   Memory growth may be expected ({growth_percentage:.1f}%)")
        else:
            print("🟢 NO MEMORY LEAK DETECTED")
            print(f"   Memory usage is stable ({growth_percentage:+.1f}%)")
        
        print()
        
        # Save detailed results
        self.save_results(leak_detected)
        
        return not leak_detected
    
    def save_results(self, leak_detected):
        """Save test results to file"""
        results_dir = Path('target/leak-test')
        results_dir.mkdir(parents=True, exist_ok=True)
        
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        results_file = results_dir / f'leak_test_{timestamp}.json'
        
        results = {
            'timestamp': datetime.now().isoformat(),
            'duration': self.duration,
            'sample_interval': self.sample_interval,
            'leak_detected': leak_detected,
            'samples': self.memory_samples,
            'summary': {
                'initial_rss': self.memory_samples[0]['rss'] if self.memory_samples else 0,
                'final_rss': self.memory_samples[-1]['rss'] if self.memory_samples else 0,
                'max_rss': max(s['rss'] for s in self.memory_samples) if self.memory_samples else 0,
                'sample_count': len(self.memory_samples)
            }
        }
        
        with open(results_file, 'w') as f:
            json.dump(results, f, indent=2)
        
        print(f"Results saved to: {results_file}")
        
        # Generate simple CSV for plotting
        csv_file = results_dir / f'leak_test_{timestamp}.csv'
        with open(csv_file, 'w') as f:
            f.write("Timestamp,RSS_MB,VMS_MB\n")
            for sample in self.memory_samples:
                f.write(f"{sample['timestamp']},{sample['rss']},{sample['vms']}\n")
        
        print(f"CSV data saved to: {csv_file}")
        print("\nTo plot the results:")
        print(f"  gnuplot -e \"set terminal png; set output 'leak_test.png'; set xlabel 'Time (s)'; set ylabel 'Memory (MB)'; plot '{csv_file}' using 1:2 with lines title 'RSS'\"")
    
    def run(self):
        """Run the complete memory leak test"""
        print("=" * 70)
        print("RustIRCd Memory Leak Detection Test")
        print("=" * 70)
        print(f"\nTest duration: {self.duration} seconds ({self.duration / 3600:.1f} hours)")
        print(f"Sample interval: {self.sample_interval} seconds")
        print(f"Server: {self.host}:{self.port}")
        print()
        
        try:
            # Start server
            if not self.start_server():
                return False
            
            # Run monitoring
            self.monitor_memory()
            
            # Analyze results
            success = self.analyze_results()
            
            return success
            
        except KeyboardInterrupt:
            print("\n\nTest interrupted by user")
            return False
        finally:
            self.stop_server()


def main():
    parser = argparse.ArgumentParser(
        description='Memory leak detection test for RustIRCd',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
  # Quick 1-hour test
  %(prog)s --duration 3600
  
  # Full 24-hour test
  %(prog)s --duration 86400
  
  # Custom sampling interval
  %(prog)s --duration 7200 --interval 30
        '''
    )
    
    parser.add_argument('--host', default='localhost', help='IRC server hostname')
    parser.add_argument('--port', type=int, default=6667, help='IRC server port')
    parser.add_argument('--duration', type=int, default=3600,
                        help='Test duration in seconds (default: 3600 = 1 hour)')
    parser.add_argument('--interval', type=int, default=60,
                        help='Memory sampling interval in seconds (default: 60)')
    
    args = parser.parse_args()
    
    test = MemoryLeakTest(
        host=args.host,
        port=args.port,
        duration=args.duration,
        sample_interval=args.interval
    )
    
    success = test.run()
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()

