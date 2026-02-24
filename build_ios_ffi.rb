#!/usr/bin/env ruby
# iOS Rust FFI Build Script
# Handles compilation of otter-mobile for iOS device + simulator architectures

require 'fileutils'
require 'json'

class IOSRustBuilder
  attr_reader :workspace_root, :deployment_target
  
  def initialize(workspace_root = File.expand_path(__dir__))
    @workspace_root = workspace_root
    @deployment_target = "12.0"
    @targets = {
      'arm64-apple-ios' => 'device',
      'aarch64-apple-ios-sim' => 'simulator',
      'x86_64-apple-ios' => 'simulator-x86'
    }
  end
  
  def build_all
    puts "🦀 Building otter-mobile for iOS..."
    install_rust_targets
    
    @targets.each do |rust_target, description|
      build_target(rust_target, description)
    end
    
    create_universal_binary if should_create_universal?
    copy_to_frameworks
  end
  
  private
  
  def install_rust_targets
    puts "📦 Installing Rust iOS targets..."
    targets = ['aarch64-apple-ios', 'x86_64-apple-ios']
    targets.each do |target|
      system("rustup target add #{target}")
    end
  end
  
  def build_target(rust_target, description)
    puts "🔨 Compiling #{description} (#{rust_target})..."
    
    unless system("cd #{workspace_root} && cargo build -p otter-mobile --target #{rust_target} --release")
      raise "Failed to build for #{rust_target}"
    end
    
    artifact = File.join(workspace_root, "target", rust_target, "release", "libotter_mobile.a")
    unless File.exist?(artifact)
      raise "Expected artifact not found: #{artifact}"
    end
    
    puts "✅ #{description}: #{artifact}"
  end
  
  def should_create_universal?
    arm64 = File.exist?(File.join(workspace_root, "target/aarch64-apple-ios/release/libotter_mobile.a"))
    sim = File.exist?(File.join(workspace_root, "target/aarch64-apple-ios-sim/release/libotter_mobile.a"))
    arm64 && sim
  end
  
  def create_universal_binary
    puts "🔗 Creating universal binary (device + simulator)..."
    
    arm64_device = File.join(workspace_root, "target/aarch64-apple-ios/release/libotter_mobile.a")
    sim_arm64 = File.join(workspace_root, "target/aarch64-apple-ios-sim/release/libotter_mobile.a")
    output = File.join(workspace_root, "target/libotter_mobile_universal.a")
    
    if File.exist?(arm64_device) && File.exist?(sim_arm64)
      system("lipo #{arm64_device} #{sim_arm64} -create -output #{output}")
      puts "✅ Universal binary: #{output}"
    end
  end
  
  def copy_to_frameworks
    puts "📂 Copying to Flutter frameworks..."
    
    flutter_app_path = File.join(workspace_root, 'flutter_app')
    frameworks_path = File.join(flutter_app_path, 'ios/Frameworks')
    FileUtils.mkdir_p(frameworks_path)
    
    # Copy individual architectures
    @targets.each do |rust_target, _|
      src = File.join(workspace_root, "target/#{rust_target}/release/libotter_mobile.a")
      if File.exist?(src)
        dst = File.join(frameworks_path, "libotter_mobile_#{rust_target.gsub('-', '_')}.a")
        FileUtils.cp(src, dst)
        puts "✅ Copied: #{File.basename(dst)}"
      end
    end
    
    # Copy universal if created
    universal = File.join(workspace_root, "target/libotter_mobile_universal.a")
    if File.exist?(universal)
      dst = File.join(frameworks_path, "libotter_mobile.a")
      FileUtils.cp(universal, dst)
      puts "✅ Copied universal: libotter_mobile.a"
    end
  end
end

if __FILE__ == $0
  builder = IOSRustBuilder.new(ARGV[0] || Dir.pwd)
  builder.build_all
end
