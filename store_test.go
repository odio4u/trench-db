package trenchdb

import (
	"os"
	"path/filepath"
	"testing"
)

type profile struct {
	Name string
	Age  int
}

func TestPutStoresByHashedKeyAndReturnsPointer(t *testing.T) {
	walPath := filepath.Join(t.TempDir(), "store.wal")
	store, err := NewStore[profile](walPath)
	if err != nil {
		t.Fatalf("NewStore() error = %v", err)
	}
	t.Cleanup(func() { _ = store.Close() })

	value := &profile{Name: "Raj", Age: 31}
	hash, err := store.Put("user-1", value)
	if err != nil {
		t.Fatalf("Put() error = %v", err)
	}

	if hash != HashKey("user-1") {
		t.Fatalf("Put() hash = %q, want %q", hash, HashKey("user-1"))
	}

	got, ok := store.GetByHashedKey(hash)
	if !ok {
		t.Fatalf("GetByHashedKey() ok = false, want true")
	}
	if got != value {
		t.Fatalf("GetByHashedKey() pointer mismatch")
	}
}

func TestStoreReplaysWALOnRestart(t *testing.T) {
	dir := t.TempDir()
	walPath := filepath.Join(dir, "store.wal")

	store, err := NewStore[profile](walPath)
	if err != nil {
		t.Fatalf("NewStore() error = %v", err)
	}

	hash, err := store.Put("user-2", &profile{Name: "Ada", Age: 29})
	if err != nil {
		t.Fatalf("Put() error = %v", err)
	}
	if err := store.Close(); err != nil {
		t.Fatalf("Close() error = %v", err)
	}

	recovered, err := NewStore[profile](walPath)
	if err != nil {
		t.Fatalf("NewStore() recovery error = %v", err)
	}
	t.Cleanup(func() { _ = recovered.Close() })

	got, ok := recovered.GetByHashedKey(hash)
	if !ok {
		t.Fatalf("GetByHashedKey() ok = false, want true")
	}
	if got.Name != "Ada" || got.Age != 29 {
		t.Fatalf("recovered value = %+v, want {Name:Ada Age:29}", *got)
	}

	content, err := os.ReadFile(walPath)
	if err != nil {
		t.Fatalf("ReadFile() error = %v", err)
	}
	if len(content) == 0 {
		t.Fatalf("WAL should not be empty")
	}
}
