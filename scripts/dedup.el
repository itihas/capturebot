(setq url-index (make-hash-table :test 'equal))
(setq title-index (make-hash-table :test 'equal))
(setq text-prefix-index (make-hash-table :test 'equal))
(defun capturebot-index-notes (files)
  (dolist (file files)
    (with-current-buffer (find-file-noselect file)
      (org-element-map (org-element-parse-buffer) '(link headline keyword)
	(lambda (elem)
	  (let ((elem-id (org-element-property-inherited :ID elem))
		(elem-type (org-element-type elem)))
	    (cond
             ((eq elem-type 'link)
              (when (member (org-element-property :type elem) '("http" "https"))
		(let ((elem-linkpath (org-element-property :path elem)))
		  (cl-pushnew elem-id (gethash elem-linkpath url-index))
		  (message "added %s : %s to url-index" elem-linkpath elem-id))))
             ((eq elem-type 'headline)
	      (let ((elem-title (org-element-property :raw-value elem)))
		(cl-pushnew elem-id (gethash elem-title title-index))
		(cl-pushnew elem-id (gethash (ntake 3 (string-split elem-title)) text-prefix-index))
		(message "added %s : %s to title-index and text-prefix-index" elem-title elem-id)))
	     ((and (eq (org-element-type elem) 'keyword)
		   (string= (org-element-property :key elem) "TITLE"))
	      (let ((elem-title (org-element-property :value elem)))
		(cl-pushnew elem-id (gethash elem-title title-index))
		(cl-pushnew elem-id (gethash (ntake 3 (string-split elem-title)) text-prefix-index))
		(message "added %s : %s to title-index and text-prefix-index" elem-title elem-id))))
	    (message "parsed %s in %s" elem-id file)))))))



(capturebot-index-notes (directory-files "~/repos/capturebot/out/" t ".*.org"))
(capturebot-index-notes (org-roam-list-files))
(defun dedup-hash (table)
  (maphash (lambda (k v) (puthash k (delete-dups v) table)) table))


(defun collisions-page ()
  (dedup-hash url-index)
  (dedup-hash title-index)
  (dedup-hash text-prefix-index)
  (with-temp-file "output.org"
		    (org-mode)
		    (insert "#+TITLE: Capturebot Duplicate Candidates\n\n")
		    ;; Helper to write collision groups
		    (cl-flet ((write-collision-group (label hash-table)
				(insert "* " label " Collisions\n\n")
				(maphash (lambda (key ids)
					   (when (> (length ids) 1)
					     (insert "** " (format "%s: %s" label key) "\n")
					     (dolist (id ids)
					       (if id (insert "   - [[id:" id "]]\n") (insert "nil")))
					     (insert "\n")))
					 hash-table)))
		      
		      (write-collision-group "URL" url-index)
		      (write-collision-group "Title" title-index)
		      (write-collision-group "Text" text-prefix-index))))

(collisions-page)

nil

nil

