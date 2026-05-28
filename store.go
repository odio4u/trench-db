package trenchdb

import (
	"bufio"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"os"
	"sync"
)

type walEntry[T any] struct {
	Op   string `json:"op"`
	Key  string `json:"key"`
	Hash string `json:"hash"`
	Data T      `json:"data"`
}

type Store[T any] struct {
	mu   sync.RWMutex
	data map[string]*T
	wal  *os.File
}

func NewStore[T any](walPath string) (*Store[T], error) {
	if walPath == "" {
		return nil, errors.New("wal path is required")
	}

	file, err := os.OpenFile(walPath, os.O_RDWR|os.O_CREATE, 0o644)
	if err != nil {
		return nil, err
	}

	s := &Store[T]{
		data: make(map[string]*T),
		wal:  file,
	}

	if err := s.replay(); err != nil {
		_ = file.Close()
		return nil, err
	}

	if _, err := file.Seek(0, 2); err != nil {
		_ = file.Close()
		return nil, err
	}

	return s, nil
}

func HashKey(key string) string {
	sum := sha256.Sum256([]byte(key))
	return hex.EncodeToString(sum[:])
}

func (s *Store[T]) Put(key string, value *T) (string, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	hash := HashKey(key)
	s.data[hash] = value

	entry := walEntry[T]{
		Op:   "put",
		Key:  key,
		Hash: hash,
	}
	if value != nil {
		entry.Data = *value
	}

	line, err := json.Marshal(entry)
	if err != nil {
		return "", err
	}
	if _, err := s.wal.Write(append(line, '\n')); err != nil {
		return "", err
	}
	if err := s.wal.Sync(); err != nil {
		return "", err
	}

	return hash, nil
}

func (s *Store[T]) GetByHashedKey(hashedKey string) (*T, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	value, ok := s.data[hashedKey]
	return value, ok
}

func (s *Store[T]) Close() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.wal == nil {
		return nil
	}
	err := s.wal.Close()
	s.wal = nil
	return err
}

func (s *Store[T]) replay() error {
	if _, err := s.wal.Seek(0, 0); err != nil {
		return err
	}

	scanner := bufio.NewScanner(s.wal)
	for scanner.Scan() {
		line := scanner.Bytes()
		if len(line) == 0 {
			continue
		}

		var entry walEntry[T]
		if err := json.Unmarshal(line, &entry); err != nil {
			return err
		}

		if entry.Op != "put" {
			continue
		}

		value := entry.Data
		v := new(T)
		*v = value
		s.data[entry.Hash] = v
	}

	return scanner.Err()
}
