// SPDX-License-Identifier: GPL-3.0-only
// Loopback-only WDA forwarding using the pinned go-ios USBMux implementation.
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"log/slog"
	"net"
	"os"
	"os/signal"
	"sync"
	"time"

	"github.com/danielpaulus/go-ios/ios"
	"github.com/danielpaulus/go-ios/ios/forward"
)

// Idle deadlines are refreshed on activity. An active HTTP connection must not
// be severed merely because 30 seconds elapsed since its creation.
type idleConn struct{ net.Conn }

func (c idleConn) Read(p []byte) (int, error) {
	if err := c.SetReadDeadline(time.Now().Add(30 * time.Second)); err != nil {
		return 0, err
	}
	return c.Conn.Read(p)
}
func (c idleConn) Write(p []byte) (int, error) {
	if err := c.SetWriteDeadline(time.Now().Add(30 * time.Second)); err != nil {
		return 0, err
	}
	return c.Conn.Write(p)
}
func serve(ctx context.Context, deviceID string) error {
	listener, err := net.Listen("tcp4", "127.0.0.1:8100")
	if err != nil {
		return errors.New("WDA loopback port is unavailable")
	}
	defer listener.Close()
	go func() { <-ctx.Done(); _ = listener.Close() }()
	slots := make(chan struct{}, 8)
	var workers sync.WaitGroup
	defer workers.Wait()
	for {
		client, err := listener.Accept()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return errors.New("WDA loopback accept failed")
		}
		select {
		case slots <- struct{}{}:
			workers.Add(1)
			go func() {
				defer workers.Done()
				defer func() { <-slots }()
				defer client.Close()
				// Windows USBMux numeric IDs change after reconnect. Resolve once per
				// TCP connection, never per HID packet or per application input event.
				device, err := ios.GetDevice(deviceID)
				if err != nil {
					return
				}
				_ = forward.StartNewProxyConnection(ctx, idleConn{client}, device.DeviceID, 8100)
			}()
		default:
			_ = client.Close()
		}
	}
}
func run() error {
	mode := flag.String("mode", "forward", "forward only")
	device := flag.String("device", "", "selected trusted iPhone")
	flag.Parse()
	if *mode != "forward" || len(*device) < 20 || len(*device) > 80 {
		return errors.New("WDA forwarding setup is invalid")
	}
	for _, c := range *device {
		if !((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F') || c == '-') {
			return errors.New("WDA device identifier is invalid")
		}
	}
	// Do not emit device IDs or pairing information from upstream logs.
	slog.SetDefault(slog.New(slog.NewTextHandler(io.Discard, nil)))
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt)
	defer cancel()
	return serve(ctx, *device)
}
func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
