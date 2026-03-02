import { faUpload } from "@fortawesome/free-solid-svg-icons/faUpload";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import type { Color } from "./types";

function FileInput({
  label,
  fileName,
  onChange,
  color,
}: {
  label: string;
  fileName?: string;
  onChange?: React.ChangeEventHandler<HTMLInputElement>;
  color?: Color;
}) {
  return (
    <div className={"file" + (fileName ? " has-name" : "") + (color ? ` ${color}` : "")}>
      <label className="file-label">
        <input className="file-input" type="file" onChange={onChange} disabled={!onChange} />
        <span className="file-cta">
          <span className="file-icon">
            <FontAwesomeIcon icon={faUpload} />
          </span>
          <span className="file-label">{label}</span>
        </span>
        {fileName && <span className="file-name">{fileName}</span>}
      </label>
    </div>
  );
}

export default FileInput;
