import type { IconLookup } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";

function Button({
  icon,
  label,
  onClick,
  color,
  disabled,
}: {
  icon?: IconLookup;
  label: string;
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
  color?: "is-success" | "is-danger" | "is-link" | "is-info";
  disabled?: boolean;
}) {
  return (
    <button className={"button" + (color ? " " + color : "")} onClick={onClick} disabled={disabled}>
      {icon && (
        <span className="icon">
          <FontAwesomeIcon icon={icon} />
        </span>
      )}
      <span>{label}</span>
    </button>
  );
}

export default Button;
