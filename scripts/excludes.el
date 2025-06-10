(defun capturebot-exclude-current-file ()
  (interactive)
  (let* ((file (dired-get-file-for-visit))
	 (message-id (with-temp-buffer
                       (insert-file-contents file)
                       (org-mode)  ; important for org functions to work
                       (org-find-property "CAPTUREBOT_MESSAGE_ID"))))
    (when message-id
      (with-temp-buffer
        (when (file-exists-p "excludes.txt")
          (insert-file-contents "excludes.txt"))
        (goto-char (point-max))
        (insert message-id "\n")
        (write-file "excludes.txt"))
      (message "Excluded message ID: %s" message-id))))
