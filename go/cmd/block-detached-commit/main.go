package main

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"runtime/debug"
	"strings"
)

// version is resolved from the embedded module version at runtime.
// Falls back to the hardcoded value for local `go run` / `go build` workflows.
var version = "0.1.0"

func init() {
	if info, ok := debug.ReadBuildInfo(); ok {
		v := info.Main.Version
		if v != "" && v != "(devel)" {
			version = strings.TrimPrefix(v, "v")
		}
	}
}

var platformMap = map[string]string{
	"linux/amd64":   "x86_64-unknown-linux-gnu",
	"linux/arm64":   "aarch64-unknown-linux-gnu",
	"darwin/amd64":  "x86_64-apple-darwin",
	"darwin/arm64":  "aarch64-apple-darwin",
	"windows/amd64": "x86_64-pc-windows-msvc",
}

func main() {
	binary, err := resolveBinary()
	if err != nil {
		fmt.Fprintf(os.Stderr, "block-detached-commit: %v\n", err)
		os.Exit(2)
	}

	cmd := exec.Command(binary, os.Args[1:]...)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		os.Exit(1)
	}
}

func resolveBinary() (string, error) {
	// 1. Binary already on PATH (e.g. installed via cargo)
	if p, err := exec.LookPath("block-detached-commit"); err == nil {
		return p, nil
	}

	// 2. Cached download
	cacheDir, err := os.UserCacheDir()
	if err != nil {
		return "", fmt.Errorf("cannot determine cache dir: %w", err)
	}

	binaryName := "block-detached-commit"
	if runtime.GOOS == "windows" {
		binaryName += ".exe"
	}

	cachedPath := filepath.Join(cacheDir, "block-detached-commit", version, binaryName)
	if _, err := os.Stat(cachedPath); err == nil {
		return cachedPath, nil
	}

	// 3. Download from GitHub releases
	return downloadBinary(cachedPath, binaryName)
}

func downloadBinary(dest, binaryName string) (string, error) {
	key := runtime.GOOS + "/" + runtime.GOARCH
	target, ok := platformMap[key]
	if !ok {
		return "", fmt.Errorf("unsupported platform %q — install manually: cargo install block-detached-commit", key)
	}

	assetName := fmt.Sprintf("%s-%s", binaryName, target)
	baseURL := fmt.Sprintf(
		"https://github.com/tupe12334/block-detached-commit/releases/download/v%s",
		version,
	)

	if err := os.MkdirAll(filepath.Dir(dest), 0755); err != nil {
		return "", fmt.Errorf("cannot create cache dir: %w", err)
	}

	fmt.Fprintf(os.Stderr, "block-detached-commit: downloading v%s for %s/%s...\n",
		version, runtime.GOOS, runtime.GOARCH)

	if err := downloadFile(baseURL+"/"+assetName, dest); err != nil {
		return "", fmt.Errorf("download failed: %w", err)
	}

	// Verify checksum
	if err := verifyChecksum(dest, baseURL+"/"+assetName+".sha256"); err != nil {
		_ = os.Remove(dest)
		return "", fmt.Errorf("checksum verification failed: %w", err)
	}

	if runtime.GOOS != "windows" {
		if err := os.Chmod(dest, 0755); err != nil {
			return "", fmt.Errorf("cannot mark binary executable: %w", err)
		}
	}

	return dest, nil
}

func downloadFile(url, dest string) error {
	resp, err := http.Get(url) //nolint:noctx
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	f, err := os.OpenFile(dest, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0644)
	if err != nil {
		return err
	}
	defer f.Close()

	_, err = io.Copy(f, resp.Body)
	return err
}

func verifyChecksum(binaryPath, checksumURL string) error {
	resp, err := http.Get(checksumURL) //nolint:noctx
	if err != nil {
		// Checksum file may not exist for pre-release builds — skip silently
		return nil
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil // skip
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return err
	}

	// Format: "<hex>  <filename>" (sha256sum output)
	expected := strings.TrimSpace(strings.Fields(string(body))[0])

	f, err := os.Open(binaryPath)
	if err != nil {
		return err
	}
	defer f.Close()

	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return err
	}
	actual := hex.EncodeToString(h.Sum(nil))

	if actual != expected {
		return fmt.Errorf("expected %s, got %s", expected, actual)
	}
	return nil
}
