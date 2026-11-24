/** Helpers for Tauri accelerator strings ("CommandOrControl+Shift+J"). */

export const isMac =
	typeof navigator !== 'undefined' &&
	/Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

const MODIFIER_CODES = new Set([
	'ControlLeft',
	'ControlRight',
	'ShiftLeft',
	'ShiftRight',
	'AltLeft',
	'AltRight',
	'MetaLeft',
	'MetaRight',
	'OSLeft',
	'OSRight'
]);

/**
 * Build an accelerator from a keydown event. Returns null while only
 * modifiers are held, or when no modifier is used (a bare key as a global
 * hotkey would swallow normal typing).
 */
export function acceleratorFromEvent(e: KeyboardEvent): string | null {
	if (MODIFIER_CODES.has(e.code)) return null;
	const parts: string[] = [];
	// The primary modifier is portable: Ctrl on Win/Linux, Cmd on macOS.
	if (isMac ? e.metaKey : e.ctrlKey) parts.push('CommandOrControl');
	if (isMac && e.ctrlKey) parts.push('Control');
	if (!isMac && e.metaKey) parts.push('Super');
	if (e.altKey) parts.push('Alt');
	if (e.shiftKey) parts.push('Shift');
	if (parts.length === 0 || (parts.length === 1 && parts[0] === 'Shift')) return null;

	let key = e.code;
	if (/^Key[A-Z]$/.test(key)) key = key.slice(3);
	else if (/^Digit\d$/.test(key)) key = key.slice(5);
	parts.push(key);
	const accel = parts.join('+');
	return isReservedAccelerator(accel) ? null : accel;
}

/** Everyday editing shortcuts that must never become the global hotkey. */
export function isReservedAccelerator(accel: string): boolean {
	return /^CommandOrControl\+(Shift\+)?[ACVXZY]$/.test(accel);
}

/** Human-readable form, e.g. "Ctrl+Shift+J" or "⌘⇧J". */
export function formatAccelerator(accel: string | null | undefined): string {
	if (!accel) return '';
	const parts = accel.split('+').map((p) => {
		switch (p) {
			case 'CommandOrControl':
			case 'CmdOrCtrl':
				return isMac ? '⌘' : 'Ctrl';
			case 'Control':
			case 'Ctrl':
				return isMac ? '⌃' : 'Ctrl';
			case 'Shift':
				return isMac ? '⇧' : 'Shift';
			case 'Alt':
				return isMac ? '⌥' : 'Alt';
			case 'Super':
				return isMac ? '⌘' : 'Super';
			default:
				return p;
		}
	});
	return parts.join(isMac ? '' : '+');
}

/**
 * Tidy a desktop portal trigger description, e.g. GNOME's "Press <Super>j"
 * becomes "Super+J".
 */
export function formatPortalTrigger(desc: string | null | undefined): string {
	if (!desc) return '';
	const m = desc.replace(/^Press\s+/i, '').match(/^((?:<[^>]+>)*)(.*)$/);
	if (!m) return desc;
	const mods = [...m[1].matchAll(/<([^>]+)>/g)].map((x) =>
		x[1] === 'Control' || x[1] === 'Primary' ? 'Ctrl' : x[1]
	);
	const key = m[2].length === 1 ? m[2].toUpperCase() : m[2];
	return [...mods, key].filter(Boolean).join('+');
}
