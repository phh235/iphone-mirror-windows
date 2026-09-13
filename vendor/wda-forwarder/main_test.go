package main

import (
	"net"
	"testing"
	"time"
)

type trackedConn struct {
	net.Conn
	reads, writes []time.Time
}

func (c *trackedConn) SetReadDeadline(t time.Time) error  { c.reads = append(c.reads, t); return nil }
func (c *trackedConn) SetWriteDeadline(t time.Time) error { c.writes = append(c.writes, t); return nil }
func (c *trackedConn) Read(p []byte) (int, error)         { p[0] = 1; return 1, nil }
func (c *trackedConn) Write(p []byte) (int, error)        { return len(p), nil }
func TestActivityRefreshesIdleDeadlines(t *testing.T) {
	tracked := &trackedConn{}
	conn := idleConn{tracked}
	p := []byte{1}
	for i := 0; i < 2; i++ {
		if _, err := conn.Read(p); err != nil {
			t.Fatal(err)
		}
		if _, err := conn.Write(p); err != nil {
			t.Fatal(err)
		}
	}
	if len(tracked.reads) != 2 || len(tracked.writes) != 2 {
		t.Fatal("deadlines were not refreshed on each operation")
	}
	if tracked.reads[1].Before(tracked.reads[0]) || tracked.writes[1].Before(tracked.writes[0]) {
		t.Fatal("deadline moved backwards")
	}
	if time.Until(tracked.reads[1]) < 29*time.Second {
		t.Fatal("idle timeout too short")
	}
}
