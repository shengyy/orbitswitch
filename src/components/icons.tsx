// Inline SVG icons drawn on a 16px grid; they inherit `currentColor`.

import type { ImgHTMLAttributes, SVGProps } from "react";
import orbitLogo from "../../assets/brand/app-icon.svg?url";

type IconProps = SVGProps<SVGSVGElement>;

/** Decorative 16px stroke icon; meaning comes from the surrounding label. */
function Icon(props: IconProps) {
  return (
    <svg
      width={16}
      height={16}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
      aria-hidden="true"
    />
  );
}

export const RefreshIcon = (props: IconProps) => (
  <Icon {...props}>
    <path d="M13.2 6.2A5.5 5.5 0 0 0 3.1 5.4M2.8 9.8a5.5 5.5 0 0 0 10.1.8" />
    <path d="M13.4 2.8v3.6H9.8M2.6 13.2V9.6h3.6" />
  </Icon>
);

export const MoreIcon = (props: IconProps) => (
  <Icon {...props} fill="currentColor" stroke="none">
    <circle cx="3.5" cy="8" r="1.25" />
    <circle cx="8" cy="8" r="1.25" />
    <circle cx="12.5" cy="8" r="1.25" />
  </Icon>
);

export const PlusIcon = (props: IconProps) => (
  <Icon {...props}>
    <path d="M8 3.25v9.5M3.25 8h9.5" />
  </Icon>
);

export const PowerIcon = (props: IconProps) => (
  <Icon {...props}>
    <path d="M8 2.25v5.5" />
    <path d="M4.6 4.2a5.25 5.25 0 1 0 6.8 0" />
  </Icon>
);

export const RestartIcon = (props: IconProps) => (
  <Icon {...props}>
    <path d="M12.9 8.6A5 5 0 1 1 11.3 4.2" />
    <path d="M12.3 2.3v2.9H9.4" />
  </Icon>
);

export const PlayIcon = (props: IconProps) => (
  <Icon {...props} fill="currentColor" stroke="none">
    <path d="M5 3.4v9.2a.6.6 0 0 0 .9.5l7.3-4.6a.6.6 0 0 0 0-1L5.9 2.9a.6.6 0 0 0-.9.5Z" />
  </Icon>
);

export const CheckIcon = (props: IconProps) => (
  <Icon {...props}>
    <path d="m3.5 8.4 2.9 2.9 6.1-6.6" />
  </Icon>
);

export const WarningIcon = (props: IconProps) => (
  <Icon {...props}>
    <path d="M7.1 2.7 1.9 11.8a1 1 0 0 0 .9 1.5h10.4a1 1 0 0 0 .9-1.5L8.9 2.7a1 1 0 0 0-1.8 0Z" />
    <path d="M8 6.3v2.9M8 11.2v.05" />
  </Icon>
);

/** Shared brand asset; the label around it supplies its accessible name. */
export const OrbitMark = (props: ImgHTMLAttributes<HTMLImageElement>) => (
  <img src={orbitLogo} width={18} height={18} {...props} alt="" aria-hidden="true" />
);
