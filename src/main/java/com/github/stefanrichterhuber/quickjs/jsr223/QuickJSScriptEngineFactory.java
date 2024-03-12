package com.github.stefanrichterhuber.quickjs.jsr223;

import java.util.List;

import javax.script.ScriptEngine;
import javax.script.ScriptEngineFactory;

public class QuickJSScriptEngineFactory implements ScriptEngineFactory {

    @Override
    public String getEngineName() {
        return "QuickJS";
    }

    @Override
    public String getEngineVersion() {
        return "1.0";
    }

    @Override
    public List<String> getExtensions() {
        return List.of("js");
    }

    @Override
    public List<String> getMimeTypes() {
        return List.of("text/javascript", "appplication/javascript");
    }

    @Override
    public List<String> getNames() {
        return List.of("javascript", "quickjs", "QuickJS");
    }

    @Override
    public String getLanguageName() {
        return "JavaScript";
    }

    @Override
    public String getLanguageVersion() {
        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'getLanguageVersion'");
    }

    @Override
    public Object getParameter(String key) {
        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'getParameter'");
    }

    @Override
    public String getMethodCallSyntax(String obj, String m, String... args) {
        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'getMethodCallSyntax'");
    }

    @Override
    public String getOutputStatement(String toDisplay) {
        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'getOutputStatement'");
    }

    @Override
    public String getProgram(String... statements) {
        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'getProgram'");
    }

    @Override
    public ScriptEngine getScriptEngine() {
        return new QuickJSScriptEngine(this);
    }

}
