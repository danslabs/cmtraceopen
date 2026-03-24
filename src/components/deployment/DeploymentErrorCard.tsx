import { tokens, Button } from "@fluentui/react-components";
import {
  useDeploymentStore,
  type DeploymentLogFile,
} from "../../stores/deployment-store";

export function DeploymentErrorCard({
  file,
  index,
}: {
  file: DeploymentLogFile;
  index: number;
}) {
  const expandedErrorIndex = useDeploymentStore((s) => s.expandedErrorIndex);
  const toggleErrorExpanded = useDeploymentStore((s) => s.toggleErrorExpanded);
  const isExpanded = expandedErrorIndex === index;

  return (
    <div
      style={{
        border: `1px solid ${tokens.colorPaletteRedBorder2}`,
        borderRadius: "4px",
        padding: "10px 12px",
        marginBottom: "8px",
        backgroundColor: tokens.colorNeutralBackground1,
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div>
          <div style={{ fontSize: "13px", fontWeight: 600 }}>
            {file.fileName}
          </div>
          <div
            style={{
              fontSize: "12px",
              color: tokens.colorNeutralForeground3,
            }}
          >
            {file.format} {file.exitCode != null && `· exit ${file.exitCode}`}
          </div>
        </div>
        {file.errorLines.length > 0 && (
          <Button
            size="small"
            appearance="subtle"
            onClick={() => toggleErrorExpanded(index)}
          >
            {isExpanded ? "Collapse" : `${file.errorLines.length} errors`}
          </Button>
        )}
      </div>

      {file.errorSummary && (
        <div
          style={{
            fontSize: "12px",
            color: tokens.colorPaletteRedForeground1,
            marginTop: "6px",
          }}
        >
          {file.errorSummary}
        </div>
      )}

      {isExpanded && file.errorLines.length > 0 && (
        <div
          style={{
            marginTop: "8px",
            padding: "8px",
            backgroundColor: tokens.colorNeutralBackground3,
            borderRadius: "3px",
            fontSize: "12px",
            fontFamily: "monospace",
            maxHeight: "200px",
            overflow: "auto",
          }}
        >
          {file.errorLines.map((line, i) => (
            <div
              key={i}
              style={{
                padding: "2px 0",
                color:
                  line.severity === "Error"
                    ? tokens.colorPaletteRedForeground1
                    : tokens.colorPaletteYellowForeground1,
              }}
            >
              <span style={{ color: tokens.colorNeutralForeground3 }}>
                L{line.lineNumber}:
              </span>{" "}
              {line.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
