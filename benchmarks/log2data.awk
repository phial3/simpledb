function to_json(record) {

    sub("%", "", record["cache-hit-ratio"]);

    fm = sprintf("{\"read\":%d,\"write\":%d}",
		 record["read-blocks"],
		 record["write-blocks"]);
    cache = sprintf("{\"hit\":%d,\"assigned\":%d,\"ratio\":%f}",
		    record["cache-hit"],
		    record["new-assigned"],
		    record["cache-hit-ratio"]);
    bm = sprintf("{\"available\":%d,\"pinned\":%d,\"unpinned\":%d,\"cache\":%s}",
		 record["available-buffers"],
		 record["pinned-buffers"],
		 record["unpinned-buffers"],
		 cache);
    item = sprintf("{\"file-manager\":%s,\"buffer-manager\":%s,\"elapsed-time\":%f}",
		   fm,
		   bm,
		   record["elapsed-time"]);

    return item;
}

/numbers of read\/written blocks:/ {
    record["read-blocks"] = $(NF-1);
    record["write-blocks"] = $(NF);
}
/numbers of available buffers:/ {
    record["available-buffers"] = $(NF);
}
/numbers of total pinned\/unpinned buffers:/ {
    record["pinned-buffers"] = $(NF-1);
    record["unpinned-buffers"] = $(NF);
}
/buffer cache hit\/assigned\(ratio\):/ {
    record["cache-hit"] = $(NF-3);
    record["new-assigned"] = $(NF-2);
    record["cache-hit-ratio"] = $(NF-1);
}
/elapsed time\(secs\):/ {
    record["elapsed-time"] = $(NF);

    # itemize and add it to summary array
    c += 1;
    summary[c] = to_json(record);
}
END {
    record["elapsed-time"] = 0.00;
    c += 1;
    summary[c] = to_json(record);

    for (i = 0; i <= c; i++) {
	print summary[i]
    }
}
