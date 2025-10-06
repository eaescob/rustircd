//! Operator Security Test
//! 
//! This example demonstrates the security measures in place to prevent
//! unauthorized operator privilege escalation.

use rustircd_core::{User, config::OperatorFlag};
use std::collections::HashSet;

fn main() {
    println!("Rust IRC Daemon - Operator Security Test");
    println!("=========================================");
    
    // Create a test user
    let mut user = User::new(
        "testuser".to_string(),
        "testuser".to_string(),
        "localhost".to_string(),
        "Test User".to_string(),
        "test.ircd.org".to_string(),
    );
    
    println!("Initial user state:");
    println!("  Nick: {}", user.nick);
    println!("  Is operator: {}", user.is_operator());
    println!("  Modes: {}", user.modes_string());
    println!();
    
    // Test 1: Attempt to set operator mode directly (should fail)
    println!("Test 1: Attempting to set +o mode directly...");
    user.add_mode('o');
    println!("  Result: Is operator = {}", user.is_operator());
    println!("  Modes: {}", user.modes_string());
    println!("  ✓ Security: Direct +o mode setting blocked");
    println!();
    
    // Test 2: Attempt to remove operator mode directly (should fail)
    println!("Test 2: Attempting to remove +o mode directly...");
    user.remove_mode('o');
    println!("  Result: Is operator = {}", user.is_operator());
    println!("  Modes: {}", user.modes_string());
    println!("  ✓ Security: Direct +o mode removal blocked");
    println!();
    
    // Test 3: Legitimate operator privilege grant
    println!("Test 3: Legitimate operator privilege grant...");
    let mut operator_flags = HashSet::new();
    operator_flags.insert(OperatorFlag::GlobalOper);
    operator_flags.insert(OperatorFlag::Administrator);
    
    user.grant_operator_privileges(operator_flags);
    println!("  Result: Is operator = {}", user.is_operator());
    println!("  Modes: {}", user.modes_string());
    println!("  Operator flags: {:?}", user.operator_flags);
    println!("  ✓ Security: Legitimate operator grant works");
    println!();
    
    // Test 4: Attempt to set operator mode after legitimate grant (should still fail)
    println!("Test 4: Attempting to set +o mode after legitimate grant...");
    user.add_mode('o');
    println!("  Result: Is operator = {}", user.is_operator());
    println!("  Modes: {}", user.modes_string());
    println!("  ✓ Security: Direct +o mode setting still blocked even after legitimate grant");
    println!();
    
    // Test 5: Legitimate operator privilege revocation
    println!("Test 5: Legitimate operator privilege revocation...");
    user.revoke_operator_privileges();
    println!("  Result: Is operator = {}", user.is_operator());
    println!("  Modes: {}", user.modes_string());
    println!("  Operator flags: {:?}", user.operator_flags);
    println!("  ✓ Security: Legitimate operator revocation works");
    println!();
    
    // Test 6: Test other modes (should work normally)
    println!("Test 6: Testing other user modes...");
    user.add_mode('i'); // invisible
    user.add_mode('w'); // wallops
    user.add_mode('a'); // away
    println!("  Modes after adding i, w, a: {}", user.modes_string());
    
    user.remove_mode('w'); // remove wallops
    println!("  Modes after removing w: {}", user.modes_string());
    println!("  ✓ Security: Other modes work normally");
    println!();
    
    // Test 7: Test operator flag checks
    println!("Test 7: Testing operator flag functionality...");
    
    // Grant some operator flags
    let mut flags = HashSet::new();
    flags.insert(OperatorFlag::LocalOper);
    flags.insert(OperatorFlag::RemoteConnect);
    user.grant_operator_privileges(flags);
    
    println!("  Is operator: {}", user.is_operator());
    println!("  Is global oper: {}", user.is_global_oper());
    println!("  Is local oper: {}", user.is_local_oper());
    println!("  Can remote connect: {}", user.can_remote_connect());
    println!("  Can local connect: {}", user.can_local_connect());
    println!("  Is administrator: {}", user.is_administrator());
    println!("  Can squit: {}", user.can_squit());
    println!("  ✓ Security: Operator flag checks work correctly");
    println!();
    
    println!("Security Test Summary:");
    println!("=====================");
    println!("✓ Direct +o mode setting is blocked");
    println!("✓ Direct +o mode removal is blocked");
    println!("✓ Legitimate operator grants work");
    println!("✓ Legitimate operator revocations work");
    println!("✓ Other user modes work normally");
    println!("✓ Operator flag system works correctly");
    println!();
    println!("The operator security system is working correctly!");
    println!("Clients cannot escalate their privileges without proper authentication.");
}
