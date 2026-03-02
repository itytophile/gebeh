interface Option<T> {
  label: string;
  value: T;
}

const WHY = "WHY LOL";

function Select<T>({
  options,
  selected,
  onClick,
}: {
  options: Option<T>[];
  selected?: string;
  onClick: (option?: Option<T>) => void;
}) {
  return (
    <div className="select">
      <select value={selected ?? WHY}>
        <option
          value={WHY}
          onClick={() => {
            onClick();
          }}
        >
          Choose a save
        </option>
        {options.map((option) => (
          <option
            key={option.label}
            value={option.label}
            onClick={() => {
              onClick(option);
            }}
          >
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}

export default Select;
