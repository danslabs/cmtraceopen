import { tokens } from "@fluentui/react-components";
import type { DeploymentLogFile } from "../../stores/deployment-store";

export function DeploymentSuccessTable({
  files,
}: {
  files: DeploymentLogFile[];
}) {
  return (
    <div
      style={{
        border: `1px solid ${tokens.colorNeutralStroke1}`,
        borderRadius: "4px",
        overflow: "hidden",
      }}
    >
      <table
        style={{
          width: "100%",
          borderCollapse: "collapse",
          fontSize: "12px",
        }}
      >
        <thead>
          <tr
            style={{
              backgroundColor: tokens.colorNeutralBackground3,
              textAlign: "left",
            }}
          >
            <th style={{ padding: "6px 10px", fontWeight: 600 }}>File</th>
            <th style={{ padding: "6px 10px", fontWeight: 600 }}>Format</th>
            <th style={{ padding: "6px 10px", fontWeight: 600 }}>Outcome</th>
            <th style={{ padding: "6px 10px", fontWeight: 600 }}>Exit Code</th>
          </tr>
        </thead>
        <tbody>
          {files.map((file) => (
            <tr
              key={file.path}
              style={{
                borderTop: `1px solid ${tokens.colorNeutralStroke2}`,
              }}
            >
              <td
                style={{
                  padding: "5px 10px",
                  fontFamily: "monospace",
                }}
              >
                {file.fileName}
              </td>
              <td style={{ padding: "5px 10px" }}>{file.format}</td>
              <td
                style={{
                  padding: "5px 10px",
                  color:
                    file.outcome === "success"
                      ? tokens.colorPaletteGreenForeground1
                      : tokens.colorPaletteYellowForeground1,
                }}
              >
                {file.outcome}
              </td>
              <td style={{ padding: "5px 10px" }}>
                {file.exitCode ?? "—"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
