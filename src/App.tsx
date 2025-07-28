import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import "./App.css";

interface SelectionEvent {
  text: string;
  app_name: string;
  timestamp: number;
  selection_type: "Selected" | "Hovered" | "Focused";
}

function App() {
  const [isDetecting, setIsDetecting] = useState(false);
  const [hasPermissions, setHasPermissions] = useState(false);
  const [selectedTexts, setSelectedTexts] = useState<SelectionEvent[]>([]);
  const [status, setStatus] = useState("Ready");

  useEffect(() => {
    // Check permissions on startup
    checkPermissions();

    // Listen for text selection changes
    const unlistenSelection = listen<SelectionEvent>(
      "text-selection-changed",
      (event) => {
        console.log("Text selection detected:", event.payload);
        setSelectedTexts((prev) => [event.payload, ...prev.slice(0, 9)]); // Keep last 10
      }
    );

    // Listen for hotkey-triggered selections
    const unlistenHotkey = listen<string>(
      "hotkey-selection-detected",
      (event) => {
        console.log("Hotkey selection:", event.payload);
        const selectionEvent: SelectionEvent = {
          text: event.payload,
          app_name: "Unknown (Hotkey)",
          timestamp: Date.now() / 1000,
          selection_type: "Selected",
        };
        setSelectedTexts((prev) => [selectionEvent, ...prev.slice(0, 9)]);
      }
    );

    return () => {
      unlistenSelection.then((f) => f());
      unlistenHotkey.then((f) => f());
    };
  }, []);

  const checkPermissions = async () => {
    try {
      const permissions = await invoke<boolean>("check_permissions");
      setHasPermissions(permissions);
      setStatus(permissions ? "Permissions granted" : "Permissions needed");
    } catch (error) {
      console.error("Error checking permissions:", error);
      setStatus("Error checking permissions");
    }
  };

  const startDetection = async () => {
    try {
      setStatus("Starting detection...");
      const result = await invoke<string>("start_text_detection");
      setIsDetecting(true);
      setStatus(result);
    } catch (error) {
      console.error("Error starting detection:", error);
      setStatus(`Error: ${error}`);
    }
  };

  const stopDetection = async () => {
    try {
      setStatus("Stopping detection...");
      const result = await invoke<string>("stop_text_detection");
      setIsDetecting(false);
      setStatus(result);
    } catch (error) {
      console.error("Error stopping detection:", error);
      setStatus(`Error: ${error}`);
    }
  };

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleTimeString();
  };

  const getTypeColor = (type: string) => {
    switch (type) {
      case "Selected":
        return "#4CAF50";
      case "Hovered":
        return "#FF9800";
      case "Focused":
        return "#2196F3";
      default:
        return "#666";
    }
  };

  return (
    <main className="container">
      <h1>ACMI Desktop - Text Selection Monitor</h1>

      <div className="control-panel">
        <div className="status">
          <strong>Status:</strong> {status}
        </div>

        <div className="permissions">
          <strong>Permissions:</strong>
          <span style={{ color: hasPermissions ? "green" : "red" }}>
            {hasPermissions ? " ✓ Granted" : " ✗ Not granted"}
          </span>
          <button onClick={checkPermissions}>Refresh</button>
        </div>

        <div className="controls">
          {!isDetecting ? (
            <button
              onClick={startDetection}
              disabled={!hasPermissions}
              className="start-btn"
            >
              Start Text Detection
            </button>
          ) : (
            <button onClick={stopDetection} className="stop-btn">
              Stop Text Detection
            </button>
          )}
        </div>

        <div className="hotkey-info">
          <p>
            <strong>Hotkey:</strong> Cmd+Shift+L (Mac) / Ctrl+Shift+L
            (Windows/Linux)
          </p>
        </div>
      </div>

      <div className="detected-texts">
        <h2>Detected Text Selections ({selectedTexts.length})</h2>
        {selectedTexts.length === 0 ? (
          <p>
            No text selections detected yet. Try selecting text in any
            application.
          </p>
        ) : (
          <div className="text-list">
            {selectedTexts.map((selection, index) => (
              <div key={index} className="text-item">
                <div className="text-header">
                  <span
                    className="selection-type"
                    style={{
                      backgroundColor: getTypeColor(selection.selection_type),
                    }}
                  >
                    {selection.selection_type}
                  </span>
                  <span className="app-name">{selection.app_name}</span>
                  <span className="timestamp">
                    {formatTimestamp(selection.timestamp)}
                  </span>
                </div>
                <div className="text-content">{selection.text}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </main>
  );
}

export default App;
