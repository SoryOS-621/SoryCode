import { useEffect, useRef } from "react";
import { Badge, Button, Dialog, Icon } from "@sorycode/ui";
import { useEngine } from "../../app/providers/EngineProvider";

/**
 * Global prompt for dangerous tool calls.
 *
 * The desktop shell emits approval requests on `sorycode://approval` and waits
 * up to five minutes; anything unanswered is denied. Denying is therefore the
 * safe default for every dismissal path (Escape, backdrop click, queue swap).
 */
export function ApprovalDialog() {
  const { approvals, respondApproval } = useEngine();
  const request = approvals[0] ?? null;
  const denyRef = useRef<HTMLButtonElement>(null);
  const requestId = request?.approvalId;

  // Focus the safe choice so an accidental Enter never grants permission.
  useEffect(() => {
    if (requestId) denyRef.current?.focus();
  }, [requestId]);

  if (!request) return null;

  const decide = (approved: boolean) => {
    void respondApproval(request.approvalId, approved);
  };

  return (
    <Dialog
      open
      onClose={() => decide(false)}
      title="Permission required"
      width={480}
      footer={
        <>
          <span className="sory-approval-queue">
            {approvals.length > 1 ? `${approvals.length} requests pending` : "Waiting for you"}
          </span>
          <Button ref={denyRef} variant="secondary" onClick={() => decide(false)}>
            Deny
          </Button>
          <Button variant="primary" icon="check" onClick={() => decide(true)}>
            Allow once
          </Button>
        </>
      }
    >
      <div className="sory-approval">
        <span className="sory-approval-icon" aria-hidden="true">
          <Icon name="alert" size={20} />
        </span>
        <div className="sory-approval-body">
          <p className="sory-approval-action">
            <code>{request.action}</code>
          </p>
          <p className="sory-approval-reason">{request.reason}</p>
          <div className="sory-approval-meta">
            <Badge tone="warning">outside the read-only policy</Badge>
            <span>Unanswered requests are denied after 5 minutes.</span>
          </div>
        </div>
      </div>
    </Dialog>
  );
}
