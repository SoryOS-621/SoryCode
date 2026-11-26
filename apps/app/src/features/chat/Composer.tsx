import { useMemo, useRef, useState, type KeyboardEvent } from "react";
import {
  Badge,
  Button,
  CommandMenu,
  IconButton,
  Kbd,
  Textarea,
  filterCommands,
  type CommandItem,
} from "@sorycode/ui";

interface SlashCommand extends CommandItem {
  action: "new" | "clear" | "settings" | "help";
}

const COMMANDS: SlashCommand[] = [
  {
    id: "/new",
    label: "/new",
    description: "Start a new session",
    icon: "plus",
    action: "new",
    group: "Session",
  },
  {
    id: "/clear",
    label: "/clear",
    description: "Clear the current conversation",
    icon: "trash",
    action: "clear",
    group: "Session",
  },
  {
    id: "/model",
    label: "/model",
    description: "Change the active model",
    icon: "sparkles",
    action: "settings",
    group: "Configuration",
  },
  {
    id: "/provider",
    label: "/provider",
    description: "Change the active provider",
    icon: "key",
    action: "settings",
    group: "Configuration",
  },
  {
    id: "/settings",
    label: "/settings",
    description: "Open settings",
    icon: "settings",
    action: "settings",
    group: "Configuration",
  },
  {
    id: "/help",
    label: "/help",
    description: "Show available commands",
    icon: "alert",
    action: "help",
    group: "Help",
  },
];

export interface ComposerProps {
  onSend: (content: string) => void | Promise<void>;
  onNewSession: () => void;
  onClear: () => void;
  onOpenSettings: () => void;
  onHelp: () => void;
  disabled?: boolean;
  busy?: boolean;
  onCancel?: () => void;
}

export function Composer({
  onSend,
  onNewSession,
  onClear,
  onOpenSettings,
  onHelp,
  disabled,
  busy,
  onCancel,
}: ComposerProps) {
  const [value, setValue] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [menuDismissed, setMenuDismissed] = useState(false);
  const textarea = useRef<HTMLTextAreaElement>(null);

  const slashQuery = value.startsWith("/") && !value.includes(" ") ? value : null;
  const items = useMemo(
    () => (slashQuery ? filterCommands(COMMANDS, slashQuery) : []),
    [slashQuery],
  );
  const menuOpen = slashQuery !== null && !menuDismissed && items.length > 0;

  const runCommand = (command: SlashCommand) => {
    setValue("");
    setActiveIndex(0);
    setMenuDismissed(false);
    switch (command.action) {
      case "new":
        onNewSession();
        break;
      case "clear":
        onClear();
        break;
      case "settings":
        onOpenSettings();
        break;
      case "help":
        onHelp();
        break;
    }
    textarea.current?.focus();
  };

  const submit = () => {
    const content = value.trim();
    if (!content || disabled) return;
    setValue("");
    setActiveIndex(0);
    setMenuDismissed(false);
    void onSend(content);
    textarea.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (menuOpen) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActiveIndex((index) => (index + 1) % items.length);
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setActiveIndex((index) => (index - 1 + items.length) % items.length);
        return;
      }
      if (event.key === "Tab") {
        event.preventDefault();
        const item = items[activeIndex];
        if (item) {
          setValue(`${item.id} `);
          setMenuDismissed(false);
        }
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        setMenuDismissed(true);
        return;
      }
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        const item = items[activeIndex];
        if (item) runCommand(item as SlashCommand);
        return;
      }
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  };

  return (
    <div className="sory-composer">
      {menuOpen ? (
        <div className="sory-composer-menu">
          <CommandMenu
            items={items}
            activeIndex={activeIndex}
            onSelect={(item) => runCommand(item as SlashCommand)}
            header="Slash commands"
            maxHeight={260}
          />
        </div>
      ) : null}

      <div className="sory-composer-box">
        <Textarea
          ref={textarea}
          rows={1}
          value={value}
          disabled={disabled}
          className="sory-composer-textarea"
          placeholder="Ask SoryCode to build, explain, refactor…  (type / for commands)"
          onChange={(event) => {
            setValue(event.target.value);
            setActiveIndex(0);
            setMenuDismissed(false);
          }}
          onKeyDown={onKeyDown}
        />
        {busy ? (
          <IconButton icon="stop" label="Stop task" onClick={onCancel} />
        ) : (
          <Button
            variant="primary"
            size="sm"
            icon="send"
            disabled={disabled || !value.trim()}
            onClick={submit}
          >
            Send
          </Button>
        )}
      </div>

      <div className="sory-composer-hints">
        <Kbd>Enter</Kbd>
        <span>send</span>
        <Kbd>Shift+Enter</Kbd>
        <span>new line</span>
        <Kbd>/</Kbd>
        <span>commands</span>
        {busy ? (
          <Badge tone="info">
            running
          </Badge>
        ) : null}
      </div>
    </div>
  );
}
