package com.github.stefanrichterhuber.quickjs;

import java.util.concurrent.CompletionStage;

public class CompletionStageWrapper<T> {

    private CompletionStage<T> stage;

    CompletionStageWrapper(final CompletionStage<T> s) {
        this.stage = s;
    }

    public CompletionStage<T> getStage() {
        return this.stage;
    }

    public void then(QuickJSFunction t) {
        stage = stage.thenApply(v -> {
            t.apply(v);
            return v;
        });
    }

    public void exceptionally(QuickJSFunction e) {
        stage = stage.exceptionally(t -> {
            final Object v = e.apply(t);
            return (T) v;
        });
    }
}
